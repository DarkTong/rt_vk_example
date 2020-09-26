extern crate ash;
pub use ash::version::{DeviceV1_0};
use ash::{util::*, vk, Device};
use std::path::Path;
use std::process::Command;

const GLSLANG_VALIDATOR: &str = "glslangValidator";
const INCLUDE_PATH: &str = "./shader/";

pub fn load_shader(device: &Device, path: &str)
-> Result<vk::ShaderModule, String>
{
    // todo：检查文件是否更改，再生成spv
    if !glsl_to_spv(path) {
        return Err(format!("failed to create spv file, {:?}", path));
    }

    let spv_path = get_spv_path(path);
    let bytes = match std::fs::read(&spv_path) {
        Ok(t) => t,
        Err(_e) => {
            return Err(format!("failed to read spv file, {:?}", path));
        }
    };
    let mut spv_file = std::io::Cursor::new(bytes);
    let code = match read_spv(&mut spv_file) {
        Ok(t) => t,
        Err(_e) => {
            return Err(format!("failed to read_spv, {}:{}", path, _e));
        }
    };
    let ci = vk::ShaderModuleCreateInfo::builder()
        .code(&code);
    unsafe {
        Ok(device.create_shader_module(&ci, None)
            .expect(&format!("shader module error, {:?}", spv_path)))
    }
}

fn get_spv_path(path: &str) -> String{
    return String::from(path) + ".spv";
}

fn glsl_to_spv(path: &str) -> bool{
    let obj = Path::new(path);
    if !obj.exists(){
        println!("current dir: {:?}", std::env::current_dir());
        return false;
    }

    let spv_path = get_spv_path(path);
    let cmd = Command::new(GLSLANG_VALIDATOR)
        .arg("-V")
        .arg(format!("-I{}", INCLUDE_PATH))
        .arg("-o")
        .arg(&spv_path)
        // .arg("-g")
        // .arg("-Od")
        .arg(path)
        .output()
        .expect(&format!("{:?} command failed to start", GLSLANG_VALIDATOR));

    println!("glsl_to_spv output: {:?}", String::from_utf8_lossy(&cmd.stdout));

    return true;
}


#[test]
fn test_glsl_to_spv()
{
    let glsl_path = format!("./shader/test/triangle.vert");
    let spv_path = format!("{}.spv", glsl_path);
    let spv_obj= std::path::Path::new(&spv_path);
    if spv_obj.exists() {
        std::fs::remove_file(&spv_path)
            .expect("删除失败？");
    }
    println!("current path: {:?}", std::env::current_dir());
    println!("glsl path: {:?}", glsl_path);
    println!("spv path: {:?}", spv_path);
    glsl_to_spv(&glsl_path);
    assert!(spv_obj.exists(), "生成spv文件失败");
}
