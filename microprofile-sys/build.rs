extern crate gcc;

use std::env;

fn main() {
    let mut config = gcc::Config::new();

    if cfg!(feature = "dx11") {
        config.define("MICROPROFILE_GPU_TIMERS_D3D11", None);
    }
    if cfg!(feature = "dx12") {
        config.define("MICROPROFILE_GPU_TIMERS_D3D12", None);
    }
    if cfg!(feature = "vulkan") {
        if let Ok(path) = std::env::var("VULKAN_SDK") {
            // TODO
            config.include(path.clone() + "/include");
            config.define("VK_NO_PROTOTYPES", None);
        }
        config.define("MICROPROFILE_GPU_TIMERS_VULKAN", None);

    }
    if cfg!(feature = "gl") {
        config.define("MICROPROFILE_GPU_TIMERS_GL", None);
    }
    config.cpp(true).file("microprofile/microprofile.cpp").compile("libmicroprofile.a");
    println!("cargo:root={}", env::var("OUT_DIR").unwrap());
}
