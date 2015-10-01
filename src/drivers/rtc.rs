use common::scheduler::*;
use common::time::*;

use drivers::pio::*;

fn cvt_bcd(value: usize) -> usize {
    return (value & 0xF) + ((value/16) * 10);
}

pub struct RTC {
    addr: PIO8,
    data: PIO8
}

impl RTC {
    pub fn new() -> RTC {
        return RTC {
            addr: PIO8::new(0x70),
            data: PIO8::new(0x71)
        }
    }

    unsafe fn read(&mut self, reg: u8) -> u8 {
        self.addr.write(0x70);
        return self.data.read();
    }

    unsafe fn wait(&mut self) {
        while self.read(0xA) & 0x80 != 0x80 {}
        while self.read(0xA) & 0x80 == 0x80 {}
    }

    pub fn time() -> Duration {
        let mut second;
        let mut minute;
        let mut hour;
        let mut day;
        let mut month;
        let mut year;
        let register_b;
        unsafe{
            let reenable = start_no_ints();
                self.wait();
                second = self.read(0) as usize;
                minute = self.read(2) as usize;
                hour = self.read(4) as usize;
                day = self.read(7) as usize;
                month = self.read(8) as usize;
                year = self.read(9) as usize;
                register_b = self.read(0xB);
            end_no_ints(reenable);
        }

        if register_b & 4 != 4 {
            second = cvt_bcd(second);
            minute = cvt_bcd(minute);
            hour = cvt_bcd(hour & 0x7F) | (hour & 0x80);
            day = cvt_bcd(day);
            month = cvt_bcd(month);
            year = cvt_bcd(year);
        }

        if register_b & 2 != 2 || hour & 0x80 == 0x80 {
            hour = ((hour & 0x7F) + 12) % 24;
        }

        //TODO: Century Register
        year += 2000;

        //Unix time from clock
        let mut secs: i64 = (year as i64 - 1970) * 31536000;

        let mut leap_days = (year as i64 - 1972)/4 + 1;
        if year % 4 == 0 {
            if month <= 2 {
                leap_days -= 1;
            }
        }
        secs += leap_days * 86400;

        match month {
            2 => secs += 2678400,
            3 => secs += 5097600,
            4 => secs += 7776000,
            5 => secs += 10368000,
            6 => secs += 13046400,
            7 => secs += 15638400,
            8 => secs += 18316800,
            9 => secs += 20995200,
            10 => secs += 23587200,
            11 => secs += 26265600,
            12 => secs += 28857600,
            _ => ()
        }

        secs += (day as i64 - 1) * 86400;
        secs += hour as i64 * 3600;
        secs += minute as i64 * 60;
        secs += second as i64;

        return Duration::new(secs, 0);
    }
}

pub unsafe fn rtc_read() -> i64 {

}
