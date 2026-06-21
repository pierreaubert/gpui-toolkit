use byteorder::{BigEndian, ReadBytesExt};
use crate :: error :: { FontLoadingError } ;
use crate::utils;
use super::consts::OTTO_HEX;
use super::consts::SFNT_HEX;
use super::consts::TRUE_HEX;
use super::consts::TYP1_HEX;
use super::font::read_number_of_fonts_from_otc_header;
use super::misc::get_slice_from_start;

pub(super) fn unpack_otc_font(data: &mut [u8], font_index: u32) -> Result<(), FontLoadingError> {
    if font_index >= read_number_of_fonts_from_otc_header(data)? {
        return Err(FontLoadingError::NoSuchFontInCollection);
    }

    let offset_table_pos_pos = 12 + 4 * font_index as usize;

    let offset_table_pos =
        get_slice_from_start(&data, offset_table_pos_pos)?.read_u32::<BigEndian>()? as usize;
    debug_assert!(utils::SFNT_VERSIONS
        .iter()
        .any(|version| { data[offset_table_pos..(offset_table_pos + 4)] == *version }));
    let num_tables = get_slice_from_start(&data, offset_table_pos + 4)?.read_u16::<BigEndian>()?;

    // Must copy forward in order to avoid problems with overlapping memory.
    let offset_table_and_table_record_size = 12 + (num_tables as usize) * 16;
    for offset in 0..offset_table_and_table_record_size {
        data[offset] = data[offset_table_pos + offset]
    }

    Ok(())
}

/// https://developer.apple.com/library/archive/documentation/mac/pdf/MoreMacintoshToolbox.pdf#page=151
pub(super) fn unpack_data_fork_font(data: &mut [u8]) -> Result<(), FontLoadingError> {
    let data_offset = (&data[..]).read_u32::<BigEndian>()? as usize;
    let map_offset = get_slice_from_start(&data, 4)?.read_u32::<BigEndian>()? as usize;
    let num_types =
        get_slice_from_start(&data, map_offset + 28)?.read_u16::<BigEndian>()? as usize + 1;

    let mut font_data_offset = 0;
    let mut font_data_len = 0;

    let type_list_offset = get_slice_from_start(&data, map_offset + 24)?.read_u16::<BigEndian>()?
        as usize
        + map_offset;
    for i in 0..num_types {
        let res_type =
            get_slice_from_start(&data, map_offset + 30 + i * 8)?.read_u32::<BigEndian>()?;

        if res_type == SFNT_HEX {
            let ref_list_offset = get_slice_from_start(&data, map_offset + 30 + i * 8 + 6)?
                .read_u16::<BigEndian>()? as usize;
            let res_data_offset =
                get_slice_from_start(&data, type_list_offset + ref_list_offset + 5)?
                    .read_u24::<BigEndian>()? as usize;
            font_data_len = get_slice_from_start(&data, data_offset + res_data_offset)?
                .read_u32::<BigEndian>()? as usize;
            font_data_offset = data_offset + res_data_offset + 4;
            let sfnt_version =
                get_slice_from_start(&data, font_data_offset)?.read_u32::<BigEndian>()?;

            // TrueType outline, 'OTTO', 'true', 'typ1'
            if sfnt_version == 0x00010000
                || sfnt_version == OTTO_HEX
                || sfnt_version == TRUE_HEX
                || sfnt_version == TYP1_HEX
            {
                break;
            }
        }
    }

    if font_data_len == 0 {
        return Err(FontLoadingError::Parse);
    }

    for offset in 0..font_data_len {
        data[offset] = data[font_data_offset + offset];
    }

    Ok(())
}
