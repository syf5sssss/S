use std::collections::HashSet;
use std::process::{ Command, Stdio };
use std::env;
use regex::Regex;
use sqlutil::{ DbHelper, FImg, Img };
use std::{ error::Error, fs, path::Path };
use tauri::{ AppHandle, Emitter };
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
mod sqlutil;

//http://asset.localhost/D:/TEST/rt2/IMG_20250209_164108/IMG_20250209_124541.jpg

//加载图片目录 scope 1:all、0:use point
#[tauri::command]
fn load_dir_imgs(app_handle: AppHandle, path: &str, scope: i32) -> Result<Vec<FImg>, String> {
    let mut imgs = Vec::<FImg>::new();
    let supported_extensions: HashSet<_> = [
        "jpg",
        "riff",
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
    let mut current_num = 0;
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
            continue; // 页面展示不出来的格式不加载。
        }
        current_num = current_num + 1;
        //读取文件查看是否存在 comment
        let p = fpath.to_string_lossy().into_owned();
        app_handle
            .emit("current_file", format!("{} -> {}", p, current_num))
            .map_err(|e| e.to_string())?;
        let cmdstr = format!("exiftool {}", p);
        println!("cmd: {:?}", cmdstr);
        let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
        let mut cmd = Command::new(shell);
        // 在 Windows 上设置不显示窗口标志
        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd
            .arg(if shell == "cmd" { "/c" } else { "-c" })
            .arg(cmdstr)
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout).to_string();
            // println!("ok: {:?}", output_str);
            let mut pic = FImg::new();
            let file_name = fpath
                .file_name()
                .and_then(|s| s.to_str()) // 将 OsStr 转换为 &str
                .ok_or_else(|| "无法获取文件名")?; // 如果失败，返回错误
            pic.name = file_name.to_string();
            pic.path = str.to_string(); //str.replace("\\", "/"); //str.to_string();
            //lng 经度{+-180},lat 纬度 {+-90} 至少lng是大的那个
            let mut f1 = 0;
            let mut f2 = 0;
            //先判断用户自定义定位信息
            if
                let Some(user_comment) = output_str.lines().find(
                    |line|
                        line
                            .splitn(2, ':')
                            .next()
                            .map(|s| s.trim()) == Some("User Comment")
                )
            {
                println!("User Comment 属性存在 ");
                f2 = f2 + 1;
                if let Some(colon_index) = user_comment.find(':') {
                    pic.comment = user_comment[colon_index + 1..].trim().to_string();
                    println!("User Comment: {:?}", pic.comment);
                    if pic.comment.is_empty() || !pic.comment.contains("baidugps:") {
                        println!("comment.is_empty || not contains baidugps:");
                    } else {
                        println!("User Comment 有值 ");
                        f2 = f2 + 1;
                        // 去掉 baidugps: 并按 : 分割
                        let trimmed_comment = pic.comment.trim_start_matches("baidugps:");
                        println!("User Comment trimmed_comment: {:?}", trimmed_comment);
                        let parts: Vec<&str> = trimmed_comment.split(',').collect();
                        println!("User Comment len: {:?}", parts.len());
                        if parts.len() >= 2 {
                            println!("User Comment start set latlng");
                            if let Ok(lat) = parts[0].parse::<f64>() {
                                pic.lat = lat;
                                println!("User Comment lat: {:?}", lat);
                            }
                            if let Ok(lng) = parts[1].parse::<f64>() {
                                pic.lng = lng;
                                println!("User Comment lng: {:?}", lng);
                            }
                        }
                    }
                } else {
                    println!("没有找到 :");
                }
            } else {
                println!("没有找到 User Comment");
            }
            //用户没有自定义,按照原图的信息读取
            if f2 < 2 {
                if
                    let Some(gps_longitude_line) = output_str.lines().find(
                        |line|
                            line
                                .splitn(2, ':')
                                .next()
                                .map(|s| s.trim()) == Some("GPS Longitude")
                    )
                {
                    f1 = f1 + 1;
                    if let Some(colon_index) = gps_longitude_line.find(':') {
                        let longitude_str = gps_longitude_line[colon_index + 1..].trim();
                        println!("longitude_str: {:?}", longitude_str);
                        if longitude_str.is_empty() {
                            println!("GPS Longitude is_empty ");
                        } else {
                            f1 = f1 + 1;
                            match parse_dms3(&longitude_str) {
                                Ok(lng) => {
                                    pic.lng = lng;
                                    println!("3 WGS84坐标:lng ({})", lng);
                                }
                                Err(e) => {
                                    println!("3 无法解析纬度 {}: {}", fpath.display(), e);
                                    match parse_dms2(&longitude_str) {
                                        Ok(lng) => {
                                            pic.lng = lng;
                                            println!("2 WGS84坐标:lng ({})", lng);
                                        }
                                        Err(e) => {
                                            println!("2 无法解析纬度 {}: {}", fpath.display(), e);
                                            match parse_dms2(&longitude_str) {
                                                Ok(lng) => {
                                                    pic.lng = lng;
                                                    println!(" WGS84坐标:lng ({})", lng);
                                                }
                                                Err(e) => {
                                                    println!(
                                                        " 无法解析纬度 {}: {}",
                                                        fpath.display(),
                                                        e
                                                    );
                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        println!("没有找到 GPS Longitude的: ");
                    }
                } else {
                    println!("没有找到 GPS Longitude ");
                }

                if
                    let Some(gps_latitude_line) = output_str.lines().find(
                        |line|
                            line
                                .splitn(2, ':')
                                .next()
                                .map(|s| s.trim()) == Some("GPS Latitude")
                    )
                {
                    if let Some(colon_index) = gps_latitude_line.find(':') {
                        let latitude_str = gps_latitude_line[colon_index + 1..].trim();
                        println!("Latitude: {:?}", latitude_str);
                        if latitude_str.is_empty() {
                            println!("GPS Latitude is_empty ");
                        } else {
                            match parse_dms3(&latitude_str) {
                                Ok(lat) => {
                                    pic.lat = lat;
                                    println!("3 WGS84坐标:lat ({})", lat);
                                }
                                Err(e) => {
                                    println!("3 无法解析纬度 {}: {}", fpath.display(), e);
                                    match parse_dms2(&latitude_str) {
                                        Ok(lat) => {
                                            pic.lat = lat;
                                            println!("2 WGS84坐标:lat ({})", lat);
                                        }
                                        Err(e) => {
                                            println!("2 无法解析纬度 {}: {}", fpath.display(), e);
                                            match parse_dms(&latitude_str) {
                                                Ok(lat) => {
                                                    pic.lat = lat;
                                                    println!("2 WGS84坐标:lat ({})", lat);
                                                }
                                                Err(e) => {
                                                    println!(
                                                        "2 无法解析纬度 {}: {}",
                                                        fpath.display(),
                                                        e
                                                    );
                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        println!("没有找到 GPS Latitude的: ");
                    }
                } else {
                    println!("没有找到 GPS Latitude ");
                }
            }
            //图片创建时间
            if
                let Some(time_comment) = output_str.lines().find(
                    |line|
                        line
                            .splitn(2, ':')
                            .next()
                            .map(|s| s.trim()) == Some("File Creation Date/Time")
                )
            {
                if let Some(colon_index) = time_comment.find(':') {
                    pic.time = time_comment[colon_index + 1..].trim().to_string();
                    println!("Create Date: {:?}", pic.time);
                    if pic.time.is_empty() {
                        println!("time.is_empty");
                    } else {
                    }
                } else {
                    println!("没有找到 :");
                }
            } else {
                println!("没有找到 Create Date");
            }
            if f2 >= 2 {
                //解析自己定义的值
                imgs.push(pic);
            } else {
                if f1 >= 2 {
                    //解析gps的值
                    imgs.push(pic);
                } else {
                    if scope == 1 {
                        imgs.push(pic);
                    }
                }
            }
        } else {
            println!("err: {:?}", String::from_utf8_lossy(&output.stderr).to_string());
            continue;
        }
    }
    println!("imgs: {:?}", imgs);
    Ok(imgs)
}

//加载没有exif信息的图片
#[tauri::command]
fn load_dir_nogpsimgs(app_handle: AppHandle, path: &str) -> Result<Vec<FImg>, String> {
    let mut imgs = Vec::<FImg>::new();
    let supported_extensions: HashSet<_> = [
        "jpg",
        "riff",
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
    let mut current_num = 0;
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
            continue; // 页面展示不出来的格式不加载。
        }
        current_num = current_num + 1;
        let p = fpath.to_string_lossy().into_owned();
        app_handle
            .emit("location_file", format!("{} -> {}", p, current_num))
            .map_err(|e| e.to_string())?;
        let cmdstr = format!("exiftool {}", p);
        println!("cmd: {:?}", cmdstr);
        let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
        let mut cmd = Command::new(shell);
        // 在 Windows 上设置不显示窗口标志
        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd
            .arg(if shell == "cmd" { "/c" } else { "-c" })
            .arg(cmdstr)
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout).to_string();
            println!("ok: {:?}", output_str);
            let mut pic = FImg::new();
            let file_name = fpath
                .file_name()
                .and_then(|s| s.to_str()) // 将 OsStr 转换为 &str
                .ok_or_else(|| "无法获取文件名")?; // 如果失败，返回错误
            pic.name = file_name.to_string();
            pic.path = str.to_string(); //str.replace("\\", "/"); //str.to_string();
            println!("pic: {:?}", pic);
            let mut f1 = 0;
            let mut f2 = 0;
            if
                let Some(gps_longitude_line) = output_str
                    .lines()
                    .find(|line| line.contains("GPS Longitude"))
            {
                println!("GPS Longitude 属性存在 ");
                f1 = f1 + 1;
                if let Some(colon_index) = gps_longitude_line.find(':') {
                    let longitude_str = gps_longitude_line[colon_index + 1..].trim();
                    println!("longitude_str: {:?}", longitude_str);
                    if longitude_str.is_empty() {
                        println!("GPS Longitude is_empty ");
                    } else {
                        println!("GPS Longitude 有值 ");
                        f1 = f1 + 1;
                    }
                } else {
                    println!("没有找到 GPS Longitude的: ");
                }
            } else {
                println!("没有找到 GPS Longitude ");
            }

            if
                let Some(user_comment) = output_str
                    .lines()
                    .find(|line| line.contains("User Comment"))
            {
                println!("User Comment 属性存在 ");
                f2 = f2 + 1;
                if let Some(colon_index) = user_comment.find(':') {
                    pic.comment = user_comment[colon_index + 1..].trim().to_string();
                    println!("User Comment: {:?}", pic.comment);
                    if pic.comment.is_empty() || !pic.comment.contains("baidugps:") {
                        println!("comment.is_empty || not contains baidugps:");
                    } else {
                        println!("User Comment 有值 ");
                        f2 = f2 + 1;
                    }
                } else {
                    println!("没有找到 :");
                }
            } else {
                println!("没有找到 User Comment");
            }
            //图片创建时间
            if
                let Some(time_comment) = output_str.lines().find(
                    |line|
                        line
                            .splitn(2, ':')
                            .next()
                            .map(|s| s.trim()) == Some("File Creation Date/Time")
                )
            {
                if let Some(colon_index) = time_comment.find(':') {
                    pic.time = time_comment[colon_index + 1..].trim().to_string();
                    println!("Create Date: {:?}", pic.time);
                    if pic.time.is_empty() {
                        println!("time.is_empty");
                    } else {
                    }
                } else {
                    println!("没有找到 :");
                }
            } else {
                println!("没有找到 Create Date");
            }
            if f1 >= 2 || f2 >= 2 {
                println!("图片已有定位信息 忽略");
            } else {
                println!("图片没有定位信息 添加");
                imgs.push(pic);
            }
        } else {
            println!("err: {:?}", String::from_utf8_lossy(&output.stderr).to_string());
            continue;
        }
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
//120 deg 7' 24.24\" E
fn parse_dms3(s: &str) -> Result<f64, Box<dyn Error>> {
    // 按空格分割字符串
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 5 {
        return Err("Invalid DMS format".into());
    }

    // 解析度
    let deg = parts[0].parse::<f64>().map_err(|e| format!("Failed to parse degrees: {}", e))?;
    // 解析分，去除单引号
    let min = parts[2]
        .trim_end_matches('\'')
        .parse::<f64>()
        .map_err(|e| format!("Failed to parse minutes: {}", e))?;
    // 解析秒，去除双引号
    let sec = parts[3]
        .trim_end_matches('"')
        .parse::<f64>()
        .map_err(|e| format!("Failed to parse seconds: {}", e))?;
    // 获取方向
    let direction = parts[4];

    // 计算十进制度数
    println!("deg: {:?}    min {:?}    sec {:?}", deg, min, sec);
    let mut lng = deg + min / 60.0 + sec / 3600.0;
    // 根据方向确定正负
    if direction == "W" || direction == "S" {
        lng = -lng;
    }

    Ok(lng)
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
            // 检查目录名称是否为 "narrow"
            if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
                if dir_name == "narrow" {
                    continue; // 跳过 "narrow" 文件夹
                }
            }
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
//已废弃 有的图片转换后 丢失了定位信息
#[tauri::command]
async fn convert_images(app_handle: AppHandle, dir: &str) -> Result<(), String> {
    println!("convert_images: {:?}", dir.to_string());
    let image_dir = Path::new(dir); // 修改为你的图片目录
    // 2025年浏览器兼容格式清单（含大小写兼容处理）
    let supported_extensions: HashSet<_> = [
        "jpg",
        "riff",
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

        // 构建命令并设置启动选项
        let mut cmd = Command::new(shell);
        cmd.arg(if shell == "cmd" { "/c" } else { "-c" })
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        // 在 Windows 上设置不显示窗口标志
        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        match cmd.status() {
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
async fn convert_image_with_metadata(app_handle: AppHandle, dir: &str) -> Result<(), String> {
    println!("convert_image_with_metadata: {:?}", dir.to_string());
    let image_dir = Path::new(dir); // 修改为你的图片目录
    // 2025年浏览器兼容格式清单（含大小写兼容处理）
    let supported_extensions: HashSet<_> = [
        "jpg",
        "riff",
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
    let current_str = image_dir.to_path_buf();
    println!("current_str: {:?}", current_str);
    app_handle
        .emit("current_file", format!("准备转换图片文件 {} 个", total))
        .map_err(|e| e.to_string())?;
    println!("发现 {} 个文件", total);
    for (index, entry) in entries.into_iter().enumerate() {
        let current_num = index + 1;
        let input_path = entry.path();
        let mut output_path = entry.path();
        let mut narrow_path = entry.path();
        println!("input_path: {:?} 处理第 {} 个文件，共{}个", input_path, current_num, total);
        let file_name = input_path
            .file_stem()
            .and_then(|n| n.to_str())
            .ok_or("Invalid filename")?;
        let currentinfo = format!("转换 {} -> {} / {} OK", file_name, current_num, total);
        app_handle
            .emit("current_file", format!("开始转换图片文件 {} ", currentinfo))
            .map_err(|e| e.to_string())?;
        // 获取 input_path 的父目录
        if let Some(parent_dir) = input_path.parent() {
            // 构建 ToJpg 文件夹的路径
            let to_jpg_dir = parent_dir.join("ToJpg");
            let to_narrow_dir = parent_dir.join("narrow");
            // 检查 ToJpg 文件夹是否存在，如果不存在则创建
            if !to_jpg_dir.exists() {
                match fs::create_dir(&to_jpg_dir) {
                    Ok(_) => println!("Created ToJpg directory: {:?}", to_jpg_dir),
                    Err(e) => eprintln!("Failed to create ToJpg directory: {}", e),
                }
            }
            // 检查 narrow 文件夹是否存在，如果不存在则创建
            if !to_narrow_dir.exists() {
                match fs::create_dir(&to_narrow_dir) {
                    Ok(_) => println!("Created narrow directory: {:?}", to_narrow_dir),
                    Err(e) => eprintln!("Failed to create narrow directory: {}", e),
                }
            }
            // 构建输出文件的路径
            let output_file_name = input_path.file_stem().unwrap_or_default().to_os_string();
            output_path = to_jpg_dir.join(output_file_name.clone()).with_extension("jpg");
            narrow_path = to_narrow_dir.join(output_file_name).with_extension("jpg");
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
        println!("narrow_path: {:?}", narrow_path);
        let command = format!("magick {} {}", input_path.display(), output_path.display());
        match runcmd(&command) {
            Ok(_) => println!("convert OK"),
            Err(e) => {
                println!("convert Failed{}", e);
                continue;
            }
        }
        app_handle
            .emit("current_file", format!("拷贝文件元数据 {} ", currentinfo))
            .map_err(|e| e.to_string())?;
        let cmdwstr = format!(
            "exiftool -overwrite_original -tagsFromFile {} {}\\ToJpg\\{}.jpg",
            input_path.display(),
            image_dir.display(),
            file_name
        );
        match runcmd(&cmdwstr) {
            Ok(_) => println!("overwrite_original OK"),
            Err(e) => {
                println!("overwrite_original Failed{}", e);
                continue;
            }
        }
        let narrowcmd = format!(
            "magick {}\\ToJpg\\{}.jpg -resize 500x500^ -gravity center -crop 500x500+0+0 +repage {}\\narrow\\{}.jpg",
            image_dir.display(),
            file_name,
            image_dir.display(),
            file_name
        );
        app_handle
            .emit("current_file", format!("{} 执行 缩略图 命令", file_name))
            .map_err(|e| e.to_string())?;
        match runcmd(&narrowcmd) {
            Ok(_) => println!("narrow OK"),
            Err(e) => eprintln!("narrow Failed{}", e),
        }
        app_handle.emit("current_file", currentinfo).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn run_command(command: &str) -> Result<String, String> {
    let path = Path::new(command);
    let p = path.to_string_lossy().into_owned();
    let cmdstr = format!("exiftool {}", p);
    println!("cmd: {:?}", cmdstr);
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };

    // 构建命令并设置启动选项
    let mut cmd = Command::new(shell);
    // 在 Windows 上设置不显示窗口标志
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd
        .arg(if shell == "cmd" { "/c" } else { "-c" })
        .arg(cmdstr)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
#[tauri::command]
fn set_location(filepath: &str, latlng: &str) -> Result<String, String> {
    let path = Path::new(filepath);
    let p = path.to_string_lossy().into_owned();
    //exiftool -UserComment=90.8897,78.1002 -charset filename=utf8 bg1.png
    let cmdstr = format!("exiftool -UserComment=baidugps:{} -overwrite_original {}", latlng, p);
    println!("cmd: {:?}", cmdstr);
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let mut cmd = Command::new(shell);
    // 在 Windows 上设置不显示窗口标志
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd
        .arg(if shell == "cmd" { "/c" } else { "-c" })
        .arg(cmdstr)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
fn runcmd(cmdstr: &str) -> Result<String, String> {
    println!("cmdstr: {:?}", cmdstr);
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let mut cmd = Command::new(shell);
    // 在 Windows 上设置不显示窗口标志
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd
        .arg(if shell == "cmd" { "/c" } else { "-c" })
        .arg(cmdstr)
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
                runcmd,
                delete_by_id,
                update_location,
                update_paths,
                setenv,
                convert_images,
                run_command,
                set_location,
                load_dir_nogpsimgs,
                convert_image_with_metadata
            ]
        )
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
