// fn read_bbt_entry(file: &File, bref: Bref) -> Result<()> {
//     //leaf bbt entry

//     Ok(())
// }

use std::{collections::HashMap, fs::File, io::{Read, Seek, SeekFrom}};

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};

use crate::{consts::MPBB_CRYPT, enums::{PropId, PropType}, structs::{Bref, ColumnEntry, PropertyEntry}};

pub fn check_magic_bytes(mut file: &File) -> bool {
    match file.seek(SeekFrom::Start(0)) {
        Ok(_) => {
            let mut buffer = vec![0u8; 4];
            match file.read_exact(&mut buffer) {
                Ok(_) => buffer == vec![33u8, 66, 68, 78],
                Err(_) => false
            }
        },
        Err(_) => false
    }
}

pub fn get_byte(mut file: &File, offset:u64) -> Result<[u8;1]> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; 1];
    let _ = file.read_exact(&mut buffer);
    Ok(buffer.try_into().unwrap())
}

pub fn get_word(mut file: &File, offset:u64) -> Result<[u8;2]> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; 2];
    let _ = file.read_exact(&mut buffer);
    Ok(buffer.try_into().unwrap())
}

// pub fn get_dword(mut file: &File, offset:u64) -> Result<[u8;4]> {
//     file.seek(SeekFrom::Start(offset))?;
//     let mut buffer = vec![0u8; 4];
//     let _ = file.read_exact(&mut buffer);
//     Ok(buffer.try_into().unwrap())
// }

pub fn get_qword(mut file: &File, offset:u64) -> Result<[u8;8]> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; 8];
    let _ = file.read_exact(&mut buffer);
    Ok(buffer.try_into().unwrap())
}

// fn get_oword(mut file: &File, offset:u64) -> Result<[u8;16]> {
//     file.seek(SeekFrom::Start(offset))?;
//     let mut buffer = vec![0u8; 16];
//     let _ = file.read_exact(&mut buffer);
//     Ok(buffer.try_into().unwrap())
// }

pub fn bid_from_u64(input:u64) -> u64 {
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/d3155aa1-ccdd-4dee-a0a9-5363ccca5352
    // first two bits should be ignored
    // Mask with all bits = 1 except the top 2 bits
    input & !(0b11 << 62)
}

pub fn get_bid(file: &File, offset:u64) -> Result<u64> {
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/d3155aa1-ccdd-4dee-a0a9-5363ccca5352
    // first two bits should be ignored
    Ok(bid_from_u64(get_u64(file, offset)?))
}

pub fn get_page(mut file: &File, offset:u64) -> Result<[u8;512]> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; 512];
    let _ = file.read_exact(&mut buffer);
    Ok(buffer.try_into().unwrap())
}

pub fn get_bref(bytes:[u8; 16]) -> Bref {
    Bref {
        bid: bid_from_u64(u64::from_le_bytes(bytes[..8].try_into().unwrap())),
        ib: u64::from_le_bytes(bytes[8..].try_into().unwrap())
    }
}

pub fn get_u8(file: &File, offset:u64) -> Result<u8> { Ok(get_byte(file, offset)?[0]) }
pub fn get_u16(file: &File, offset:u64) -> Result<u16> { Ok(u16::from_le_bytes(get_word(file, offset)?)) }
// pub fn get_u32(file: &File, offset:u64) -> Result<u32> { Ok(u32::from_le_bytes(get_dword(file, offset)?)) }
pub fn get_u64(file: &File, offset:u64) -> Result<u64> { Ok(u64::from_le_bytes(get_qword(file, offset)?)) }


pub fn get_tables() -> (&'static [u8; 256], &'static [u8; 256]) {
    let r = MPBB_CRYPT[..256].try_into().unwrap();
    let i = MPBB_CRYPT[512..768].try_into().unwrap();
    (r, i)
}

pub fn decode_permute(data: &mut [u8]) {
    let (_, inverse) = get_tables();

    for byte in data.iter_mut() {
        *byte = inverse[*byte as usize];
    }
}
// pub fn encode_permute(data: &mut [u8]) {
//     let (forward, _) = get_tables();

//     for byte in data.iter_mut() {
//         *byte = forward[*byte as usize];
//     }
// }

pub fn get_column_entry_string(column_entries: &HashMap<PropId, ColumnEntry>, prop_id: PropId, irow: usize) -> Result<String> {
    match column_entries.get(&prop_id) {
        Some(column_entry) => {
            match column_entry.value_string.as_ref() {
                Some(rows) => {
                    Ok(rows[irow].clone())
                },
                None => {
                    bail!("no rows of type string in the column entry {:?}", column_entry)
                }
            }
        },
        None => {
            bail!("no column {:?} in column_entries", prop_id)
        }
    }
}

pub fn get_column_entry_i32(column_entries: &HashMap<PropId, ColumnEntry>, prop_id: PropId, irow: usize) -> Result<i32> {
    match column_entries.get(&prop_id) {
        Some(column_entry) => {
            match column_entry.value_i32.as_ref() {
                Some(rows) => {
                    Ok(rows[irow])
                },
                None => {
                    bail!("no rows of type i32 in the column entry {:?}", column_entry)
                }
            }
        },
        None => {
            bail!("no column {:?} in column_entries", prop_id)
        }
    }
}

pub fn get_prop_string(property_entries: &HashMap<PropId, PropertyEntry>, prop_id: &PropId) -> String {
	match property_entries.get(&prop_id) {
		Some(property_entry) => {
			match property_entry.property.prop_type {
				PropType::Binary => String::from_utf8_lossy(&property_entry.value_binary.as_ref().expect("Do not expect a binary type to have None value_binary.")).to_string(),
				_ => property_entry.value_string.clone()
			}
		},
		None => String::new()
	}
}

pub fn get_prop_datetime_op(property_entries: &HashMap<PropId, PropertyEntry>, prop_id: &PropId) -> Option<DateTime<Utc>> {
	match property_entries.get(&prop_id) {
		Some(property_entry) => property_entry.value_datetime,
		None => None
	}
}
