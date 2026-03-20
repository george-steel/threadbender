pub const INCLUDES: &[(&str, &str)] = &[
    ("global.wgsl", include_str!("global.wgsl")),
    ("spirals.wgsl", euler_spirals::SHADER_INCLUDE),
];

pub const GRID: &str = include_str!("grid.wgsl");

pub const HANDLES: &str = include_str!("handles.wgsl");

pub const SPIRAL_TEST: &str = include_str!("spiral_test.wgsl");

pub const SPLINE_PLOT: &str = include_str!("spline_plot.wgsl");

