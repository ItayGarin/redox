use alloc::boxed::Box;

use collections::string::ToString;
use collections::vec::Vec;

use core::cmp;

use common::debug;
use common::to_num::ToNum;

use network::common::*;
use network::ethernet::*;

use fs::{KScheme, Resource, Url};

use system::error::{Error, Result, ENOENT};

/// A ethernet resource
pub struct EthernetResource {
    /// The network
    network: Box<Resource>,
    /// The data
    data: Vec<u8>,
    /// The MAC addresss
    peer_addr: MacAddr,
    /// The ethernet type
    ethertype: u16,
}

impl Resource for EthernetResource {
    fn dup(&self) -> Result<Box<Resource>> {
        match self.network.dup() {
            Ok(network) => Ok(box EthernetResource {
                network: network,
                data: self.data.clone(),
                peer_addr: self.peer_addr,
                ethertype: self.ethertype,
            }),
            Err(err) => Err(err),
        }
    }

    fn path(&self, buf: &mut [u8]) -> Result<usize> {
        let path_string = format!("ethernet:{}/{:X}", self.peer_addr.to_string(), self.ethertype);
        let path = path_string.as_bytes();

        for (b, p) in buf.iter_mut().zip(path.iter()) {
            *b = *p;
        }

        Ok(cmp::min(buf.len(), path.len()))
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        debugln!("      Read ethernet:{}/{:X}: {}", self.peer_addr.to_string(), self.ethertype, buf.len());

        while self.data.is_empty() {
            let mut bytes = [0; 65536];
            let count = try!(self.network.read(&mut bytes));
            if let Some(frame) = EthernetII::from_bytes(bytes[.. count].to_vec()) {
                if frame.header.ethertype.get() == self.ethertype && (unsafe { frame.header.dst.equals(MAC_ADDR) }
                    || frame.header.dst.equals(BROADCAST_MAC_ADDR)) && (frame.header.src.equals(self.peer_addr)
                    || self.peer_addr.equals(BROADCAST_MAC_ADDR))
                {
                    self.data = frame.data;
                    break;
                } else {
                    debugln!("      Ignore ethernet:{}/{:X}: {}", self.peer_addr.to_string(), self.ethertype, count);
                }
            }
        }

        // TODO: Allow splitting
        let mut i = 0;
        while i < buf.len() && i < self.data.len() {
            buf[i] = self.data[i];
            i += 1;
        }

        self.data.clear();

        debugln!("      Return ethernet:{}/{:X}: {}", self.peer_addr.to_string(), self.ethertype, i);
        return Ok(i);
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        debugln!("      Write ethernet:{}/{:X}: {}", self.peer_addr.to_string(), self.ethertype, buf.len());

        let data = Vec::from(buf);

        self.network.write(&EthernetII {
            header: EthernetIIHeader {
                src: unsafe { MAC_ADDR },
                dst: self.peer_addr,
                ethertype: n16::new(self.ethertype),
            },
            data: data,
        }.to_bytes()).and(Ok(buf.len()))
    }

    fn sync(&mut self) -> Result<()> {
        self.network.sync()
    }
}

pub struct EthernetScheme;

impl KScheme for EthernetScheme {
    fn scheme(&self) -> &str {
        "ethernet"
    }

    fn open(&mut self, url: Url, _: usize) -> Result<Box<Resource>> {
        let parts: Vec<&str> = url.reference().split("/").collect();
        if let Some(host_string) = parts.get(0) {
            if let Some(ethertype_string) = parts.get(1) {
                if let Ok(mut network) = Url::from_str("network:").unwrap().open() {
                    let ethertype = ethertype_string.to_num_radix(16) as u16;

                    if !host_string.is_empty() {
                        return Ok(box EthernetResource {
                            network: network,
                            data: Vec::new(),
                            peer_addr: MacAddr::from_str(host_string),
                            ethertype: ethertype,
                        });
                    } else {
                        loop {
                            let mut bytes = [0; 8192];
                            match network.read(&mut bytes) {
                                Ok(count) => {
                                    if let Some(frame) = EthernetII::from_bytes(bytes[.. count].to_vec()) {
                                        if frame.header.ethertype.get() == ethertype &&
                                           (unsafe { frame.header.dst.equals(MAC_ADDR) } ||
                                            frame.header.dst.equals(BROADCAST_MAC_ADDR)) {
                                            return Ok(box EthernetResource {
                                                network: network,
                                                data: frame.data,
                                                peer_addr: frame.header.src,
                                                ethertype: ethertype,
                                            });
                                        }
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    }
                } else {
                    debug::d("Ethernet: Failed to open network:\n");
                }
            } else {
                debug::d("Ethernet: No ethertype provided\n");
            }
        } else {
            debug::d("Ethernet: No host provided\n");
        }

        Err(Error::new(ENOENT))
    }
}
