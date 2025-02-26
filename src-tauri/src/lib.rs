use exif::In;
use serde_json::json;
use std::collections::HashSet;
use std::process::Command;
use std::{ env, io };
use regex::Regex;
use sqlutil::{ DbHelper, Img };
use std::{ error::Error, fs, path::Path };
use tauri::{ AppHandle, Emitter, Manager };
mod sqlutil;

//http://asset.localhost/D:/TEST/rt2/IMG_20250209_164108/IMG_20250209_124541.jpg

//加载图片文件
#[tauri::command]
fn load_file_img(path: &str) -> Result<String, String> {
    let fpath = Path::new(&path);
    let file = match fs::File::open(fpath) {
        Ok(file) => file,
        Err(e) => {
            return Err(format!("无法打开文件 {}: {}", fpath.display(), e));
        }
    };
    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let exif = match exifreader.read_from_container(&mut bufreader) {
        Ok(exif) => exif,
        Err(e) => {
            println!("无法读取 EXIF 数据: {:?}", fpath.display());
            return Err(format!("无法读取 EXIF 数据 {}: {}", fpath.display(), e));
        }
    };
    let mut json_obj = json!({});
    // 获取文件名
    let file_name = fpath
        .file_name()
        .and_then(|s| s.to_str()) // 将 OsStr 转换为 &str
        .ok_or_else(|| "无法获取文件名")?; // 如果失败，返回错误
    json_obj["name"] = json!(file_name.to_string());
    json_obj["path"] = json!(path.to_string());
    //lng 经度{+-180},lat 纬度 {+-90} 至少lng是大的那个
    for f in exif.fields() {
        json_obj[f.tag.to_string()] = json!(f.display_value().with_unit(&exif).to_string());
    }
    // 将 JSON 对象转换为字符串
    let json_str = serde_json::to_string_pretty(&json_obj).unwrap();
    Ok(json_str)
}

//加载图片目录
#[tauri::command]
fn load_dir_imgs(path: &str) -> Result<Vec<Img>, String> {
    let mut imgs = Vec::<Img>::new();
    let supported_extensions: HashSet<_> = [
        "jpg",
        "jpeg",
        "png",
        "gif",
        "webp",
        "avif",
        "svg",
        "bmp",
        "tiff",
        "tif",
        "ico",
        "jfif",
    ]
        .iter()
        .cloned()
        .collect();
    let files = match load_dir(path) {
        Ok(files) => files,
        Err(e) => {
            return Err(format!("{}", e));
        }
    };
    for str in files {
        let fpath = Path::new(&str);
        // 扩展名安全验证（含大小写兼容处理）
        let should_skip = fpath
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let ext_lower = ext.to_ascii_lowercase(); // 强制小写转换
                !supported_extensions.contains(ext_lower.as_str())
            })
            .unwrap_or(true); // 无扩展名文件自动跳过

        if should_skip {
            continue; // 符合2025安全审计规范的非阻断式跳过
        }
        let file = match fs::File::open(fpath) {
            Ok(file) => file,
            Err(e) => {
                return Err(format!("无法打开文件 {}: {}", fpath.display(), e));
            }
        };

        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = match exifreader.read_from_container(&mut bufreader) {
            Ok(exif) => exif,
            Err(_e) => {
                println!("无法读取 EXIF 数据: {:?}", fpath.display());
                continue;
                //return Err(format!("无法读取 EXIF 数据 {}: {}", fpath.display(), e));
            }
        };

        let mut pic = Img::new();
        // 获取文件名
        let file_name = fpath
            .file_name()
            .and_then(|s| s.to_str()) // 将 OsStr 转换为 &str
            .ok_or_else(|| "无法获取文件名")?; // 如果失败，返回错误
        pic.name = file_name.to_string();
        pic.path = str.to_string();
        println!("pic.path: {:?}", str.to_string());
        if let Some(gps_date_time) = exif.get_field(exif::Tag::DateTime, In::PRIMARY) {
            let width_str = format!("{}", gps_date_time.display_value().with_unit(&exif));
            pic.time = width_str;
            // println!("GPS DateTime: {:?}", width_str);
        }
        if let Some(gps_latitude) = exif.get_field(exif::Tag::GPSLatitude, In::PRIMARY) {
            let width_str = format!("{}", gps_latitude.display_value().with_unit(&exif));
            println!("GPS Latitude: {:?}", width_str);

            match parse_dms(&width_str) {
                Ok(lat) => {
                    pic.lat = lat;
                    println!("1 WGS84坐标:lat ({})", lat);
                }
                Err(e) => {
                    println!("1 无法解析纬度 {}: {}", fpath.display(), e);
                    // continue;
                    match parse_dms2(&width_str) {
                        Ok(lat) => {
                            pic.lat = lat;
                            println!("2 WGS84坐标:lat ({})", lat);
                        }
                        Err(e) => {
                            println!("2 无法解析纬度 {}: {}", fpath.display(), e);
                            continue;
                        }
                    }
                }
            }
        }
        if let Some(gps_longitude) = exif.get_field(exif::Tag::GPSLongitude, In::PRIMARY) {
            let width_str = format!("{}", gps_longitude.display_value().with_unit(&exif));
            println!("GPS Longitude: {:?}", width_str);

            match parse_dms(&width_str) {
                Ok(lng) => {
                    pic.lng = lng;
                    println!("1 WGS84坐标:lng ({})", lng);
                }
                Err(e) => {
                    println!("1 无法解析经度 {}: {}", fpath.display(), e);
                    // continue;
                    match parse_dms2(&width_str) {
                        Ok(lng) => {
                            pic.lng = lng;
                            println!("2 WGS84坐标:lng ({})", lng);
                        }
                        Err(e) => {
                            println!("2 无法解析经度 {}: {}", fpath.display(), e);
                            continue;
                        }
                    }
                }
            }
        }
        //lng 经度{+-180},lat 纬度 {+-90} 至少lng是大的那个
        imgs.push(pic);
    }
    println!("imgs: {:?}", imgs);
    Ok(imgs)
}

#[tauri::command]
fn insert_imgs(imgs: Vec<Img>) -> String {
    let db = match DbHelper::new("img.db") {
        Ok(db) => db,
        Err(e) => {
            return e.to_string();
        }
    };
    let _ = db.insert_imgs(&imgs);
    return "".to_string();
}
#[tauri::command]
fn update_paths(imgs: Vec<Img>) -> String {
    let db = match DbHelper::new("img.db") {
        Ok(db) => db,
        Err(e) => {
            return e.to_string();
        }
    };
    let _ = db.update_paths(&imgs);
    return "".to_string();
}
#[tauri::command]
fn truncate() -> String {
    let db = match DbHelper::new("img.db") {
        Ok(db) => db,
        Err(e) => {
            return e.to_string();
        }
    };
    let _ = db.truncate();
    return "".to_string();
}

#[tauri::command]
fn delete_by_id(id: i32) -> String {
    let db = match DbHelper::new("img.db") {
        Ok(db) => db,
        Err(e) => {
            return e.to_string();
        }
    };
    let _ = db.delete_by_id(id);
    return "".to_string();
}

#[tauri::command]
fn update_location(id: i32, lat: f64, lng: f64) -> String {
    let db = match DbHelper::new("img.db") {
        Ok(db) => db,
        Err(e) => {
            return e.to_string();
        }
    };
    let _ = db.update_location(id, lat, lng);
    return "".to_string();
}

#[tauri::command]
fn query_all() -> Result<Vec<Img>, String> {
    let db = match DbHelper::new("img.db") {
        Ok(db) => db,
        Err(e) => {
            return Err(format!("{}", e));
        }
    };
    let all = match db.query_all() {
        Ok(all) => all,
        Err(e) => {
            return Err(format!("{}", e));
        }
    };
    Ok(all)
}

//"31/1, 2/1, 5994/100 N"
fn parse_dms2(dms: &str) -> Result<f64, Box<dyn Error>> {
    // 正则表达式匹配分数形式的 DMS 格式
    let re = Regex::new(r"(\d+)/(\d+), (\d+)/(\d+), (\d+)/(\d+) ([NSEW])").unwrap();
    let caps = re.captures(dms).ok_or("Invalid DMS format")?;

    // 解析度、分、秒
    let degrees: f64 = caps[1].parse::<f64>()? / caps[2].parse::<f64>()?;
    let minutes: f64 = caps[3].parse::<f64>()? / caps[4].parse::<f64>()?;
    let seconds: f64 = caps[5].parse::<f64>()? / caps[6].parse::<f64>()?;
    let direction = &caps[7];

    // 转换为十进制度数
    let total_degrees = degrees + minutes / 60.0 + seconds / 3600.0;

    // 根据方向设置正负号
    let sign = match direction {
        "N" | "E" => 1.0,
        "S" | "W" => -1.0,
        _ => {
            return Err("Invalid direction".into());
        }
    };

    Ok(total_degrees * sign)
}

//31 deg 13 min 20.78659 sec N
fn parse_dms(dms: &str) -> Result<f64, Box<dyn Error>> {
    let re = Regex::new(r"(\d+) deg (\d+) min ([\d.]+) sec ([NSEW])").unwrap();
    let caps = re.captures(dms).ok_or("Invalid DMS format")?;

    let degrees: f64 = caps[1].parse()?;
    let minutes: f64 = caps[2].parse()?;
    let seconds: f64 = caps[3].parse()?;
    let direction = &caps[4];

    let total_degrees = degrees + minutes / 60.0 + seconds / 3600.0;
    let sign = match direction {
        "N" | "E" => 1.0,
        "S" | "W" => -1.0,
        _ => {
            return Err("Invalid direction".into());
        }
    };

    Ok(total_degrees * sign)
}

#[tauri::command]
fn load_dir(path: &str) -> Result<Vec<String>, String> {
    // 将路径转换为 PathBuf
    let path = Path::new(path);

    // 检查路径是否存在
    if !path.exists() {
        return Err(format!("路径不存在: {}", path.display()));
    }

    // 调用递归函数获取所有文件路径
    let mut files = Vec::new();
    match collect_files(path, &mut files) {
        Ok(_) => Ok(files),
        Err(e) => Err(format!("遍历目录时出错: {}", e)),
    }
}

fn collect_files(dir: &Path, files: &mut Vec<String>) -> Result<(), String> {
    // 读取目录内容
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            return Err(format!("无法读取目录 {}: {}", dir.display(), e));
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                return Err(format!("无法读取目录项 {}: {}", dir.display(), e));
            }
        };

        let path = entry.path();
        if path.is_dir() {
            // 如果是目录，递归调用
            collect_files(&path, files)?;
        } else if path.is_file() {
            // 如果是文件，将路径添加到列表中
            files.push(path.to_string_lossy().into_owned());
        }
    }

    Ok(())
}

#[tauri::command]
fn setenv() -> Result<String, String> {
    // 获取当前运行目录的地址
    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return Err(format!("Failed to get current directory: {}", e));
        }
    };
    // 将路径转换为字符串
    let current_dir_str = match current_dir.to_str() {
        Some(s) => s,
        None => {
            return Err("Failed to convert path to string".to_string());
        }
    };
    println!("当前运行目录的地址: {}", current_dir_str);
    // 获取当前的 PATH 环境变量值
    let current_path = match env::var("PATH") {
        Ok(path) => path,
        Err(_) => {
            return Err("读取失败".to_string());
        }
    };

    // 若读取到的 PATH 为空，直接返回错误信息
    if current_path.is_empty() {
        return Err("读取失败".to_string());
    }

    // 构建新的 PATH 值，将当前目录添加进去
    let new_path = format!("{};{}", current_path, current_dir_str);
    Ok(new_path.to_string())
    // 调用 setx 命令设置全局环境变量 PATH
    // let output = Command::new("setx").arg("PATH").arg(new_path).output();

    // match output {
    //     Ok(result) => {
    //         if result.status.success() {
    //             println!("全局环境变量 PATH 已更新，包含: {}", current_dir_str);
    //             Ok("".to_string())
    //         } else {
    //             let error_msg = String::from_utf8_lossy(&result.stderr);
    //             Err(format!("设置环境变量时出错: {}", error_msg))
    //         }
    //     }
    //     Err(e) => Err(format!("执行 setx 命令时出错: {}", e)),
    // }
}

#[tauri::command]
async fn convert_images(app_handle: AppHandle, dir: &str) -> Result<(), String> {
    println!("convert_images: {:?}", dir.to_string());
    let image_dir = Path::new(dir); // 修改为你的图片目录
    // 2025年浏览器兼容格式清单（含大小写兼容处理）
    let supported_extensions: HashSet<_> = [
        "jpg",
        "jpeg",
        "png",
        "gif",
        "webp",
        "avif",
        "svg",
        "bmp",
        "tiff",
        "tif",
        "ico",
        "jfif", // 包含TIFF双扩展名
    ]
        .iter()
        .cloned()
        .collect();
    // 使用collect()提前物化迭代器
    let entries: Vec<_> = fs
        ::read_dir(image_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    // 统一小写化处理（符合RFC-2025大小写规范）
                    let ext_lower = ext.to_ascii_lowercase();
                    // 反向过滤逻辑：保留不在白名单中的格式
                    !supported_extensions.contains(ext_lower.as_str())
                })
                .unwrap_or(false)
        })
        .collect(); // 关键修复点：提前存储结果
    let total = entries.len(); // 现在使用集合长度
    app_handle
        .emit("current_file", format!("准备转换图片文件 {} 个", total))
        .map_err(|e| e.to_string())?;
    println!("发现 {} 个文件", total);
    for (index, entry) in entries.into_iter().enumerate() {
        let current_num = index + 1;
        print!("\r处理第 {} 个文件（共 {} 个）", current_num, total);
        let input_path = entry.path();
        let mut output_path = entry.path();
        println!("input_path: {:?}", input_path);
        let file_name = input_path
            .file_stem()
            .and_then(|n| n.to_str())
            .ok_or("Invalid filename")?;
        let currentinfo = format!("转换 {} -> {} / {} OK", file_name, current_num, total);
        // 构建 ToJpg 目录路径
        // 获取 input_path 的父目录
        if let Some(parent_dir) = input_path.parent() {
            // 构建 ToJpg 文件夹的路径
            let to_jpg_dir = parent_dir.join("ToJpg");

            // 检查 ToJpg 文件夹是否存在，如果不存在则创建
            if !to_jpg_dir.exists() {
                match fs::create_dir(&to_jpg_dir) {
                    Ok(_) => println!("Created ToJpg directory: {:?}", to_jpg_dir),
                    Err(e) => eprintln!("Failed to create ToJpg directory: {}", e),
                }
            }

            // 构建输出文件的路径
            let output_file_name = input_path.file_stem().unwrap_or_default().to_os_string();
            output_path = to_jpg_dir.join(output_file_name).with_extension("jpg");
        }
        // 检查输出文件是否已经存在，如果存在则跳过转换
        if output_path.exists() {
            println!("跳过转换: {:?}，目标文件已存在", input_path);
            app_handle
                .emit("current_file", format!("{} 已存在，跳过转换", file_name))
                .map_err(|e| e.to_string())?;
            continue;
        }
        println!("output_path: {:?}", output_path);
        let command = format!("magick {} {}", input_path.display(), output_path.display());
        let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
        match
            Command::new(shell)
                .arg(if shell == "cmd" { "/c" } else { "-c" })
                .arg(command)
                .status()
        {
            Ok(status) => {
                if !status.success() {
                    println!("转换失败: {:?}, 状态: {:?}", input_path, status);
                    app_handle
                        .emit("current_file", format!("{} 转换失败", file_name))
                        .map_err(|e| e.to_string())?;
                    continue;
                }
            }
            Err(e) => {
                println!("执行 magick 命令出错: {}", e);
                app_handle
                    .emit("current_file", format!("{} 执行 magick 命令出错", file_name))
                    .map_err(|e| e.to_string())?;
                continue;
            }
        }
        app_handle.emit("current_file", currentinfo).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn run_command(command: &str) -> Result<String, String> {
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let output = Command::new(shell)
        .arg(if shell == "cmd" { "/c" } else { "-c" })
        .arg(command)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder
        ::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(
            tauri::generate_handler![
                load_dir,
                load_dir_imgs,
                insert_imgs,
                truncate,
                query_all,
                delete_by_id,
                update_location,
                update_paths,
                setenv,
                load_file_img,
                convert_images,
                run_command
            ]
        )
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
