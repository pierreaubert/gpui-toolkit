use std :: os :: raw :: { c_char } ;
use super::types::BDF_PropertyType;

pub(super) const PS_DICT_FULL_NAME: u32 = 38;

pub(super) const TT_NAME_ID_FULL_NAME: u16 = 4;

pub(super) const TT_PLATFORM_APPLE_UNICODE: u16 = 0;

pub(super) const FT_POINT_TAG_ON_CURVE: c_char = 0x01;

pub(super) const FT_POINT_TAG_CUBIC_CONTROL: c_char = 0x02;

pub(super) const OS2_FS_SELECTION_OBLIQUE: u16 = 1 << 9;

#[allow(dead_code)]
const BDF_PROPERTY_TYPE_NONE: BDF_PropertyType = 0;

#[allow(dead_code)]
pub(super) const BDF_PROPERTY_TYPE_ATOM: BDF_PropertyType = 1;

#[allow(dead_code)]
const BDF_PROPERTY_TYPE_INTEGER: BDF_PropertyType = 2;

#[allow(dead_code)]
const BDF_PROPERTY_TYPE_CARDINAL: BDF_PropertyType = 3;

