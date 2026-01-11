#[cfg(test)]
mod tests {
    use crate::gpu3d::camera::Camera3D;
    use glam::Vec3;

    #[test]
    fn test_top_down_view_projection() {
        // Setup camera looking from +Y down to origin (Top View)
        // In our coordinate system:
        // X: [-1, 1] (Freq)
        // Y: [-0.5, 0.5] (SPL/Height)
        // Z: [-1, 1] (Angle)

        let mut camera = Camera3D::new();
        // Position high up on Y axis
        camera.position = Vec3::new(0.0, 10.0, 0.0);
        camera.target = Vec3::new(0.0, 0.0, 0.0);
        camera.up = Vec3::Z; // Need to define up vector. If looking down Y, Z up means Z axis aligns with screen Y?
        // Wait, standard UP is usually Y. If looking down, UP cannot be Y.
        // Let's say UP is -Z (standard map view? North is Up?)
        // If UP is -Z, then Z axis points DOWN on screen.
        // If UP is +Z, then Z axis points UP on screen.
        // If we want Z axis (Angle) to be horizontal?
        // Wait. X is usually horizontal.
        // If X is Right, Z is Up (on screen).
        // Then UP should be Z.
        camera.up = Vec3::new(0.0, 0.0, -1.0); // Let's try standard 3D generic convention where Z is "depth".

        // Actually, let's just use what works for us.
        // If we look from (0, 10, 0) to (0,0,0).
        // View matrix LookAt.

        let width = 1000.0;
        let height = 1000.0;

        // Let's project corners of the box top face (Y=0.5)
        let corners = [
            Vec3::new(-1.0, 0.5, -1.0), // Back Left
            Vec3::new(1.0, 0.5, -1.0),  // Back Right
            Vec3::new(1.0, 0.5, 1.0),   // Front Right
            Vec3::new(-1.0, 0.5, 1.0),  // Front Left
        ];

        for &corner in &corners {
            let screen_pos = camera.project_to_screen(corner, width, height);
            assert!(screen_pos.is_some(), "Point {:?} should be visible", corner);
            let s = screen_pos.unwrap();

            // Should be within screen bounds
            assert!(s.x >= 0.0 && s.x <= width, "X out of bounds: {}", s.x);
            assert!(s.y >= 0.0 && s.y <= height, "Y out of bounds: {}", s.y);
        }

        // Check alignment
        // If UP is -1.0 Z.
        // Camera Forward is -Y (0, -1, 0).
        // Right = Forward x Up = (0,-1,0) x (0,0,-1) = (1, 0, 0) = +X. Correct.
        // New Up (Camera Y) = Right x Forward = (1,0,0) x (0,-1,0) = (0,0,-1).

        // So on screen:
        // Screen X aligns with World X.
        // Screen Y aligns with World -Z (because Camera Y is -Z).
        // So standard top-down map view.

        // Center (0, 0.5, 0) should be at center of screen (500, 500)
        let center = camera
            .project_to_screen(Vec3::new(0.0, 0.5, 0.0), width, height)
            .unwrap();
        assert!((center.x - 500.0).abs() < 1.0);
        assert!((center.y - 500.0).abs() < 1.0);

        // Z Label for 0 degrees (Z=0, after mapping [0,1]->[-1,1]?? No Z=0 is center)
        // Normalized 0.5 -> mapped to 0.0.
        // 0 degrees is at Z=0.
        // Should be at screen center Y.
        let pos_0 = camera
            .project_to_screen(Vec3::new(1.0, 0.0, 0.0), width, height)
            .unwrap();
        assert!(
            (pos_0.y - 500.0).abs() < 10.0,
            "Z=0 should be near center Y"
        );

        // Z Label for -180 (Z=-1).
        // Camera Y ends up pointing -Z.
        // So Z=-1 corresponds to Camera Y = +1 (Up).
        // Screen Y=0 is top.
        // So Z=-1 should be near Y=0 (Top of screen).
        let pos_n180 = camera
            .project_to_screen(Vec3::new(1.0, 0.0, -1.0), width, height)
            .unwrap();
        assert!(pos_n180.y < 500.0, "Z=-1 should be above center");

        // Z Label for 180 (Z=1).
        // Should be near Y=1000 (Bottom).
        let pos_180 = camera
            .project_to_screen(Vec3::new(1.0, 0.0, 1.0), width, height)
            .unwrap();
        assert!(pos_180.y > 500.0, "Z=1 should be below center");

        // Ordering check: -180 (top) -> 0 (center) -> 180 (bottom)
        assert!(pos_n180.y < pos_0.y, "-180 should be above 0");
        assert!(pos_0.y < pos_180.y, "0 should be above 180");

        println!(
            "Test passed: -180 at {}, 0 at {}, 180 at {}",
            pos_n180.y, pos_0.y, pos_180.y
        );
    }

    #[test]
    fn test_xz_plane_projection() {
        // Test looking at the X/Z plane (Freq/Angle)
        // Camera at (0, 10, 0) looking at (0, 0, 0) with Z pointing Down on screen?
        // Let's verify the rectangle of the plot floor.

        let mut camera = Camera3D::new();
        camera.position = Vec3::new(0.0, 5.0, 0.0);
        camera.target = Vec3::new(0.0, 0.0, 0.0);
        // Up is -Z, so Z axis points down on screen
        camera.up = Vec3::new(0.0, 0.0, -1.0);

        // Orthographic projection might be easier to reason about for "matching rectangle",
        // but we use perspective. With target at center, it should still look like a rectangle/trapezoid symmetric.

        let width = 800.0;
        let height = 600.0;

        // Check corners of the floor (Y = -0.5)
        let floor_y = -0.5;
        let corners = [
            Vec3::new(-1.0, floor_y, -1.0), // Min Freq, Min Angle (-180)
            Vec3::new(1.0, floor_y, -1.0),  // Max Freq, Min Angle
            Vec3::new(1.0, floor_y, 1.0),   // Max Freq, Max Angle (180)
            Vec3::new(-1.0, floor_y, 1.0),  // Min Freq, Max Angle
        ];

        let s_corners: Vec<Vec3> = corners
            .iter()
            .map(|&p| camera.project_to_screen(p, width, height).unwrap())
            .collect();

        // Check symmetry
        // Top-Left vs Top-Right (Y should be same, X symmetric)
        // Note: With UP=-Z:
        // Z=-1 (Min Angle) is "Up" in camera space -> High Y in camera -> Low Y on screen (0 is top).
        // So Z=-1 should have smaller Screen Y than Z=1.

        // Corner 0: (-1, -1) -> Left, Top
        // Corner 1: (1, -1) -> Right, Top
        // Corner 2: (1, 1) -> Right, Bottom
        // Corner 3: (-1, 1) -> Left, Bottom

        // Verify Y coordinates (Top edge)
        assert!(
            (s_corners[0].y - s_corners[1].y).abs() < 1.0,
            "Top edge should be horizontal"
        );
        // Verify Y coordinates (Bottom edge)
        assert!(
            (s_corners[2].y - s_corners[3].y).abs() < 1.0,
            "Bottom edge should be horizontal"
        );

        // Verify X coordinates (Left edge)
        assert!(
            (s_corners[0].x - s_corners[3].x).abs() < 1.0,
            "Left edge should be vertical"
        );
        // Verify X coordinates (Right edge)
        assert!(
            (s_corners[1].x - s_corners[2].x).abs() < 1.0,
            "Right edge should be vertical"
        );

        // Verify Relative Positions
        assert!(
            s_corners[0].x < s_corners[1].x,
            "Min Freq should be Left of Max Freq"
        );
        assert!(
            s_corners[0].y < s_corners[3].y,
            "Min Angle (-180) should be Above Max Angle (180) on screen"
        );

        println!("Freq/Angle Plane check passed. Corners form a valid rectangle on screen.");
    }

    #[test]
    fn test_zoom_consistency() {
        // Verify that zooming (changing distance) behaves consistently in projection
        let mut camera = Camera3D::new();
        camera.target = Vec3::ZERO;
        camera.up = Vec3::Y;
        camera.fov = 45.0_f32.to_radians();
        camera.aspect = 1.0;

        let width = 800.0;
        let height = 800.0;
        let center_x = width * 0.5;

        // Position 1: Distance 10
        camera.position = Vec3::new(0.0, 0.0, 10.0);

        let p_world = Vec3::new(1.0, 0.0, 0.0);
        let p_screen_1 = camera.project_to_screen(p_world, width, height).unwrap();

        // Position 2: Distance 5 (Zoom in 2x)
        camera.position = Vec3::new(0.0, 0.0, 5.0);
        let p_screen_2 = camera.project_to_screen(p_world, width, height).unwrap();

        let dist_1 = p_screen_1.x - center_x;
        let dist_2 = p_screen_2.x - center_x;

        println!("Zoom consistency: Dist1 = {}, Dist2 = {}", dist_1, dist_2);

        // With perspective, if we halve the distance to proper plane, the object should appear roughly twice as large.
        // At dist 10, Z depth is 10. At dist 5, Z depth is 5.
        // Projection ~ X / Z.
        // So 1/10 vs 1/5. 0.1 vs 0.2.
        // So Dist2 should be ~2.0 * Dist1.

        let ratio = dist_2 / dist_1;
        assert!(
            (ratio - 2.0).abs() < 0.1,
            "Zooming in 2x should double the screen size of objects on plane. Ratio was {}",
            ratio
        );
    }
}
