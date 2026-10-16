//! The Direct3D 11 demo (`D3DDemo/` upstream, the namespace `GpuInterop.D3DDemo`): a coloured
//! cube drawn with Direct3D 11 into textures shared with the compositor (Windows).

#[cfg(windows)]
pub mod d3d11;
mod d3d11_demo_control;
#[cfg(windows)]
mod d3d11_swapchain;
#[cfg(windows)]
mod d3d_content;

pub use d3d11_demo_control::{create_look_at_left_handed, create_perspective_field_of_view_left_handed, D3D11DemoControl};
#[cfg(windows)]
pub use d3d11_swapchain::{D3D11Swapchain, D3D11SwapchainImage};
#[cfg(windows)]
pub use d3d_content::D3DContent;

use ferroui_base::TypeInfo;

/// The classes of the namespace `GpuInterop.D3DDemo` (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[D3D11DemoControl::TYPE];
