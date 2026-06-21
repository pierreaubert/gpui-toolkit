use cocoa :: { foundation :: { NSUInteger } } ;

pub(super) type NSDragOperation = NSUInteger;

#[derive(PartialEq)]
pub enum UserTabbingPreference {
    Never,
    Always,
    InFullScreen,
}

