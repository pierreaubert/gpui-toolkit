use freetype_sys :: { FT_Face , FT_Long , FT_Set_Char_Size } ;

pub(super) unsafe fn setup_freetype_face(face: FT_Face) {
    reset_freetype_face_char_size(face);
}

pub(super) unsafe fn reset_freetype_face_char_size(face: FT_Face) {
    // Apple Color Emoji has 0 units per em. Whee!
    let units_per_em = (*face).units_per_EM as i64;
    if units_per_em > 0 {
        assert_eq!(
            FT_Set_Char_Size(face, ((*face).units_per_EM as FT_Long) << 6, 0, 0, 0),
            0
        );
    }
}

