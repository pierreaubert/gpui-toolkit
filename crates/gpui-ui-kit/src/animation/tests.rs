use super::Animation;
use super::easing::ease;
use super::easing::interpolate;
use super::keyframe_animation::KeyframeAnimation;
use super::keyframe_animation::evaluate_keyframes;
use super::spring::Spring;
use super::types::Easing;
use std::time::Duration;

#[test]
fn test_linear_easing() {
    assert_eq!(ease(Easing::Linear, 0.0), 0.0);
    assert_eq!(ease(Easing::Linear, 0.5), 0.5);
    assert_eq!(ease(Easing::Linear, 1.0), 1.0);
}

#[test]
fn test_ease_in_out_bounds() {
    for easing in [
        Easing::EaseInQuad,
        Easing::EaseOutQuad,
        Easing::EaseInOutQuad,
        Easing::EaseInCubic,
        Easing::EaseOutCubic,
        Easing::EaseInOutCubic,
    ] {
        let start = ease(easing, 0.0);
        let end = ease(easing, 1.0);
        assert!(
            (start - 0.0).abs() < 0.001,
            "{:?} at t=0 should be ~0, got {}",
            easing,
            start
        );
        assert!(
            (end - 1.0).abs() < 0.001,
            "{:?} at t=1 should be ~1, got {}",
            easing,
            end
        );
    }
}

#[test]
fn test_ease_out_faster_start() {
    // EaseOut should have faster progress at the start
    let ease_in = ease(Easing::EaseInQuad, 0.5);
    let ease_out = ease(Easing::EaseOutQuad, 0.5);
    assert!(ease_out > ease_in, "EaseOut should be ahead at midpoint");
}

#[test]
fn test_back_overshoot() {
    // EaseOutBack should overshoot past 1.0 before settling
    let peak = ease(Easing::EaseOutBack, 0.7);
    assert!(peak > 1.0, "EaseOutBack should overshoot");
}

#[test]
fn test_animation_progress() {
    let anim = Animation::new().duration_ms(1000);

    assert_eq!(anim.progress(Duration::from_millis(0)), 0.0);
    // At 50% time with EaseOutQuad, progress is 0.75 (faster at start)
    let mid_progress = anim.progress(Duration::from_millis(500));
    assert!(mid_progress > 0.5, "EaseOut should be past 50% at midpoint");
    assert!(mid_progress < 0.9, "But not too far");
    assert_eq!(anim.progress(Duration::from_millis(1000)), 1.0);
    assert_eq!(anim.progress(Duration::from_millis(2000)), 1.0);
}

#[test]
fn test_animation_with_delay() {
    let anim = Animation::new().duration_ms(1000).delay_ms(500);

    assert_eq!(anim.progress(Duration::from_millis(0)), 0.0);
    assert_eq!(anim.progress(Duration::from_millis(500)), 0.0);
    assert!(anim.progress(Duration::from_millis(1000)) > 0.4);
    assert_eq!(anim.progress(Duration::from_millis(1500)), 1.0);
}

#[test]
fn test_animation_is_complete() {
    let anim = Animation::new().duration_ms(1000).delay_ms(500);

    assert!(!anim.is_complete(Duration::from_millis(0)));
    assert!(!anim.is_complete(Duration::from_millis(1000)));
    assert!(anim.is_complete(Duration::from_millis(1500)));
    assert!(anim.is_complete(Duration::from_millis(2000)));
}

#[test]
fn test_animation_repeat_replays_each_cycle() {
    let anim = Animation::new()
        .duration_ms(100)
        .easing(Easing::Linear)
        .repeat(1);

    assert_eq!(anim.total_duration(), Duration::from_millis(200));
    assert_eq!(anim.progress(Duration::from_millis(50)), 0.5);
    assert_eq!(anim.progress(Duration::from_millis(100)), 0.0);
    assert_eq!(anim.progress(Duration::from_millis(150)), 0.5);
    assert_eq!(anim.progress(Duration::from_millis(200)), 1.0);
    assert!(!anim.is_complete(Duration::from_millis(199)));
    assert!(anim.is_complete(Duration::from_millis(200)));
}

#[test]
fn test_animation_alternate_reverses_and_ends_at_start_after_odd_repeat() {
    let anim = Animation::new()
        .duration_ms(100)
        .easing(Easing::Linear)
        .repeat(1)
        .alternate(true);

    assert_eq!(anim.progress(Duration::from_millis(100)), 1.0);
    assert_eq!(anim.progress(Duration::from_millis(150)), 0.5);
    assert_eq!(anim.progress(Duration::from_millis(200)), 0.0);
    assert!(anim.is_complete(Duration::from_millis(200)));
}

#[test]
fn test_spring_settling() {
    let spring = Spring::default();
    let mut pos = 0.0;
    let mut vel = 0.0;
    let target = 100.0;
    let dt = 1.0 / 60.0; // 60 FPS

    // Simulate for 2 seconds
    for _ in 0..120 {
        (pos, vel) = spring.step(pos, target, vel, dt);
    }

    // Should be close to target
    assert!(
        (pos - target).abs() < 1.0,
        "Spring should settle near target"
    );
    assert!(vel.abs() < 1.0, "Spring velocity should be near zero");
}

#[test]
fn test_interpolate() {
    let result = interpolate(0.0, 100.0, Easing::Linear, 0.5);
    assert_eq!(result, 50.0);

    let result = interpolate(0.0, 100.0, Easing::Linear, 0.0);
    assert_eq!(result, 0.0);

    let result = interpolate(0.0, 100.0, Easing::Linear, 1.0);
    assert_eq!(result, 100.0);
}

#[test]
#[should_panic(expected = "spring mass must be positive")]
fn test_spring_zero_mass_panics() {
    Spring::new(100.0, 10.0, 0.0);
}

#[test]
fn test_spring_force_zero_mass_safe() {
    // Direct field mutation bypasses the new() assert; force() should
    // still be safe via the max(1e-6) clamp.
    let spring = Spring {
        stiffness: 100.0,
        damping: 10.0,
        mass: 0.0,
    };
    let f = spring.force(10.0, 5.0);
    assert!(f.is_finite(), "force should be finite even with mass=0");
}

#[test]
fn test_keyframe_animation() {
    let anim = KeyframeAnimation::new()
        .at(0.0, 0.0_f32)
        .at(0.5, 50.0)
        .at(1.0, 100.0);

    let result = evaluate_keyframes(&anim, 0.25, |a, b, t| a + (b - a) * t);
    assert!(result.is_some());
    assert!((result.unwrap() - 25.0).abs() < 0.1);

    let result = evaluate_keyframes(&anim, 0.75, |a, b, t| a + (b - a) * t);
    assert!(result.is_some());
    assert!((result.unwrap() - 75.0).abs() < 0.1);
}
