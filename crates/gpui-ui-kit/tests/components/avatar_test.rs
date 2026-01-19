//! Avatar component tests

use gpui_ui_kit::avatar::{Avatar, AvatarGroup, AvatarShape, AvatarSize, AvatarStatus};

#[test]
fn test_avatar_creation() {
    let avatar = Avatar::new()
        .name("John Doe")
        .src("https://example.com/avatar.png");
    drop(avatar);
}

#[test]
fn test_avatar_sizes() {
    let sizes = [
        AvatarSize::Xs,
        AvatarSize::Sm,
        AvatarSize::Md,
        AvatarSize::Lg,
        AvatarSize::Xl,
        AvatarSize::Xxl,
    ];

    for size in &sizes {
        let avatar = Avatar::new().size(*size);
        drop(avatar);
    }
}

#[test]
fn test_avatar_shapes() {
    let circle = Avatar::new().shape(AvatarShape::Circle);
    drop(circle);

    let square = Avatar::new().shape(AvatarShape::Square);
    drop(square);
}

#[test]
fn test_avatar_status() {
    let statuses = [
        AvatarStatus::Online,
        AvatarStatus::Offline,
        AvatarStatus::Away,
        AvatarStatus::Busy,
    ];

    for status in &statuses {
        let avatar = Avatar::new().status(*status);
        drop(avatar);
    }
}

#[test]
fn test_avatar_group() {
    let avatars = vec![
        Avatar::new().name("A"),
        Avatar::new().name("B"),
        Avatar::new().name("C"),
    ];

    let group = AvatarGroup::new()
        .avatars(avatars)
        .max_display(2)
        .size(AvatarSize::Sm);

    drop(group);
}
