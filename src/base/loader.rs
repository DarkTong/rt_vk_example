use ash::{util::*, vk};
use std::fs;
use std::path::Path;
use std::io;
use std::process::Command;

const GLSLANG_VALIDATOR: &str = "glslangValidator";
const INCLUDE_PATH: &str = "../shader/"

pub fn LoadShader(path: &str)
-> vk::ShaderModule
{

}

fn glsl_to_spv(path: &str) -> bool
{
    let obj = Path::new(path);
    if !obj.exists(){
        return false;
    }

    let spv_path = String::from(path) + ".spv";
    Command::new(GLSLANG_VALIDATOR)
        .arg("-V")
        .arg(foramt!("-I {}", INCLUDE_PATH))
        .arg(format!("-o {}", spv_path))
        .arg("-g")
        .arg("-Od")
        .arg(format!("{}", path))
        .spawn()
        .expect(format!("{} command failed to start", GLSLANG_VALIDATOR));

    return true;
}
