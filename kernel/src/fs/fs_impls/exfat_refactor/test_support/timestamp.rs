// SPDX-License-Identifier: MPL-2.0

use core::time::Duration;

use time::{Date, PrimitiveDateTime, Time, UtcOffset};

pub(in super::super) fn encode_exfat_date(date: Date) -> u16 {
    let year = u16::try_from(date.year() - 1980).unwrap();
    let month = u16::from(u8::from(date.month()));
    let day = u16::from(date.day());
    (year << 9) | (month << 5) | day
}

pub(in super::super) fn encode_exfat_date_only(date: Date) -> [u8; 4] {
    let date_bytes = encode_exfat_date(date).to_le_bytes();
    [0, 0, date_bytes[0], date_bytes[1]]
}

pub(in super::super) fn encode_exfat_date_time(date: Date, time: Time) -> ([u8; 4], u8) {
    assert_eq!(time.second() % 2, 0);
    assert_eq!(time.millisecond() % 10, 0);

    let encoded_time = (u16::from(time.hour()) << 11)
        | (u16::from(time.minute()) << 5)
        | u16::from(time.second() / 2);
    let time_bytes = encoded_time.to_le_bytes();
    let date_bytes = encode_exfat_date(date).to_le_bytes();
    let ten_ms_increment = u8::try_from(time.millisecond() / 10).unwrap();
    (
        [time_bytes[0], time_bytes[1], date_bytes[0], date_bytes[1]],
        ten_ms_increment,
    )
}

pub(in super::super) fn encode_valid_utc_offset_byte(offset: UtcOffset) -> u8 {
    let quarter_hours = offset.whole_seconds() / (15 * 60);
    assert!((-64..=63).contains(&quarter_hours));
    0x80 | (u8::try_from(quarter_hours.rem_euclid(128)).unwrap() & 0x7f)
}

pub(in super::super) fn expected_timestamp(date: Date, time: Time, offset: UtcOffset) -> Duration {
    let timestamp = PrimitiveDateTime::new(date, time).assume_offset(offset);
    Duration::from_nanos(u64::try_from(timestamp.unix_timestamp_nanos()).unwrap())
}
