use syn::{GenericArgument, PathArguments, Type};

pub(super) fn option_inner_type(ty: &Type) -> Option<Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segments = &type_path.path.segments;
    let is_std_option = match segments.len() {
        1 => segments[0].ident == "Option",
        3 => {
            (segments[0].ident == "std" || segments[0].ident == "core")
                && segments[1].ident == "option"
                && segments[2].ident == "Option"
        }
        _ => false,
    };
    if !is_std_option {
        return None;
    }
    let segment = segments.last()?;
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    if args.args.len() != 1 {
        return None;
    }
    let Some(GenericArgument::Type(inner)) = args.args.first() else {
        return None;
    };
    Some(inner.clone())
}
