// ==================== 跨平台缓存机制 ====================

// 全局缓存目录路径
lazy_static::lazy_static! {
    static ref CACHE_DIR_PATH: Option<PathBuf> = initialize_cache_directory();
}

// 缓存进程图标信息，包含上次更新时间，以提高性能
// 键是进程路径的MD5值，值是(图标数据, 时间戳, 是否有图标)的元组
lazy_static::lazy_static! {
    static ref ICON_CACHE: Mutex<HashMap<String, (Option<String>, SystemTime, bool)>> = Mutex::new(HashMap::new());
}

// 初始化缓存目录
fn initialize_cache_directory() -> Option<PathBuf> {
    // 使用用户的 home 目录下的 .portview 目录
    let cache_dir = dirs::home_dir().map(|home| home.join(".sniffer"));

    let cache_dir = match cache_dir {
        Some(dir) => dir,
        None => {
            eprintln!("Failed to get home directory");
            return None;
        }
    };

    eprintln!("Cache directory: {:?}", cache_dir);

    // 创建缓存目录（如果不存在）
    match std::fs::create_dir_all(&cache_dir) {
        Ok(_) => {
            eprintln!("Successfully created cache directory: {:?}", cache_dir);

            // 预加载缓存文件到内存中
            preload_cache_files(&cache_dir);

            Some(cache_dir)
        }
        Err(e) => {
            eprintln!("Failed to create cache directory {:?}: {}", cache_dir, e);
            None
        }
    }
}

// 预加载缓存文件到内存中
fn preload_cache_files(cache_dir: &PathBuf) {
    // 读取缓存目录中的所有PNG文件
    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                // 检查是否为PNG文件
                if path.extension().map_or(false, |ext| ext == "png") {
                    if let Some(file_name) = path.file_stem() {
                        if let Some(file_name_str) = file_name.to_str() {
                            // 尝试读取文件内容
                            if let Ok(file_content) = std::fs::read(&path) {
                                // 将文件内容编码为base64并存储到缓存中
                                let base64_icon = base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &file_content,
                                );

                                // 将缓存数据添加到全局缓存中
                                let mut cache = ICON_CACHE.lock().unwrap();
                                // 对于预加载的文件，我们假设它们都是有效的图标
                                cache.insert(
                                    file_name_str.to_string(), // 使用文件名（不含扩展名）作为键
                                    (Some(base64_icon), SystemTime::now(), true),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

// 获取缓存目录路径
fn get_cache_directory() -> Option<PathBuf> {
    CACHE_DIR_PATH.clone()
}

// 通用的获取进程图标函数（使用缓存）
pub fn get_process_icon_by_path(exe_path: &str) -> Option<String> {
    // 使用路径的MD5值作为缓存键
    let cache_key = format!("{:x}", md5::compute(exe_path.as_bytes()));

    // 一次获取锁，检查缓存
    {
        let cache = ICON_CACHE.lock().unwrap();
        if let Some((cached_icon, _, has_cached)) = cache.get(&cache_key) {
            // 如果之前已经缓存了"无图标"的结果，则直接返回None
            if !has_cached {
                return None;
            }
            // 返回缓存的图标
            return cached_icon.clone();
        }
    }

    // 使用路径的MD5值作为缓存文件名
    let icon_filename = format!("{}.png", cache_key);

    // 获取预初始化的缓存目录
    let cache_dir = get_cache_directory()?;

    // 检查是否已经为该进程路径生成过缓存文件
    let png_cache_path = cache_dir.join(icon_filename);

    // 如果缓存文件存在，直接使用它
    if png_cache_path.exists() {
        if let Ok(png_data) = std::fs::read(&png_cache_path) {
            let base64_icon = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &png_data,
            );

            // 将图标添加到内存缓存
            let mut cache = ICON_CACHE.lock().unwrap();
            cache.insert(
                cache_key,
                (Some(base64_icon.clone()), SystemTime::now(), true),
            );

            return Some(base64_icon);
        }
    }

    // 缓存文件不存在，提取图标
    let png_data = extract_icon_from_exe(exe_path)?;

    // 保存转换后的PNG到缓存目录
    let base64_icon = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &png_data,
    );

    // 尝试保存到文件缓存
    let cache_result = std::fs::write(&png_cache_path, &png_data);

    // 将图标添加到内存缓存
    let mut cache = ICON_CACHE.lock().unwrap();
    cache.insert(
        cache_key,
        (Some(base64_icon.clone()), SystemTime::now(), true),
    );

    // 如果文件缓存失败，记录警告
    if cache_result.is_err() {
        eprintln!("Warning: Failed to write icon cache file: {:?}", png_cache_path);
    }

    Some(base64_icon)
}

// 从可执行文件提取图标（平台特定实现）
fn extract_icon_from_exe(exe_path: &str) -> Option<Vec<u8>> {
    #[cfg(target_os = "windows")]
    {
        extract_icon_from_exe_windows(exe_path)
    }

    #[cfg(target_os = "macos")]
    {
        extract_icon_from_exe_macos(exe_path)
    }

    #[cfg(target_os = "linux")]
    {
        extract_icon_from_exe_linux(exe_path)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

// ==================== Windows 平台实现 ====================

#[cfg(target_os = "windows")]
fn extract_icon_from_exe_windows(exe_path: &str) -> Option<Vec<u8>> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use winapi::shared::minwindef::HINSTANCE;
    use winapi::um::shellapi::ExtractIconW;
    use winapi::um::winuser::DestroyIcon;

    // 将路径转换为宽字符字符串
    let wide_path: Vec<u16> = OsStr::new(exe_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // 使用ExtractIconW从EXE文件中提取图标
    unsafe {
        let h_instance = ptr::null_mut() as HINSTANCE;
        let h_icon = ExtractIconW(
            h_instance,
            wide_path.as_ptr(),
            0, // 第一个图标
        );

        // 检查图标句柄是否有效
        if h_icon as usize > 1 {
            // 0和1是特殊值，表示没有图标或错误
            // 尝试将图标转换为图像数据
            let icon_data = extract_icon_to_png(h_icon);

            // 销毁图标句柄
            DestroyIcon(h_icon);

            return icon_data;
        } else {
            // 销毁无效图标句柄
            if h_icon as usize > 1 {
                DestroyIcon(h_icon);
            }
        }
    }

    // 缓存找不到图标的结果
    let cache_key = format!("{:x}", md5::compute(exe_path.as_bytes()));
    let mut cache = ICON_CACHE.lock().unwrap();
    cache.insert(cache_key, (None, SystemTime::now(), false));

    None
}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;
// 辅助函数：将图标转换为PNG数据
#[cfg(target_os = "windows")]
use winapi::shared::windef::HICON;

#[cfg(target_os = "windows")]
unsafe fn extract_icon_to_png(h_icon: HICON) -> Option<Vec<u8>> {
    use std::mem;
    use std::ptr;
    use winapi::um::wingdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDIBits, SelectObject,
        BI_RGB, DIB_RGB_COLORS,
    };
    use winapi::um::winnt::HANDLE;
    use winapi::um::winuser::{DrawIconEx, GetDC, GetIconInfo, ReleaseDC};

    // 获取图标信息
    let mut icon_info: winapi::um::winuser::ICONINFO = mem::zeroed();
    if GetIconInfo(h_icon, &mut icon_info) == 0 {
        return None;
    }

    // 获取屏幕DC
    let hdc_screen = GetDC(ptr::null_mut());
    if hdc_screen.is_null() {
        // 清理资源
        if !icon_info.hbmColor.is_null() {
            DeleteObject(icon_info.hbmColor as *mut _);
        }
        if !icon_info.hbmMask.is_null() {
            DeleteObject(icon_info.hbmMask as *mut _);
        }
        return None;
    }

    // 创建兼容DC
    let hdc_mem = CreateCompatibleDC(hdc_screen);
    if hdc_mem.is_null() {
        ReleaseDC(ptr::null_mut(), hdc_screen);
        if !icon_info.hbmColor.is_null() {
            DeleteObject(icon_info.hbmColor as *mut _);
        }
        if !icon_info.hbmMask.is_null() {
            DeleteObject(icon_info.hbmMask as *mut _);
        }
        return None;
    }

    // 选择位图到内存DC
    let hbm_old = SelectObject(hdc_mem, icon_info.hbmColor as *mut _);

    // 获取颜色位图信息
    let mut bitmap: winapi::um::wingdi::BITMAP = mem::zeroed();
    if winapi::um::wingdi::GetObjectW(
        icon_info.hbmColor as HANDLE,
        mem::size_of::<winapi::um::wingdi::BITMAP>() as i32,
        &mut bitmap as *mut _ as *mut _,
    ) == 0
    {
        SelectObject(hdc_mem, hbm_old);
        DeleteDC(hdc_mem);
        ReleaseDC(ptr::null_mut(), hdc_screen);
        if !icon_info.hbmColor.is_null() {
            DeleteObject(icon_info.hbmColor as *mut _);
        }
        if !icon_info.hbmMask.is_null() {
            DeleteObject(icon_info.hbmMask as *mut _);
        }
        return None;
    }

    let width = bitmap.bmWidth;
    let height = bitmap.bmHeight.abs();

    // 准备BITMAPINFO结构
    let mut bmi: winapi::um::wingdi::BITMAPINFO = mem::zeroed();
    bmi.bmiHeader.biSize = mem::size_of::<winapi::um::wingdi::BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width;
    bmi.bmiHeader.biHeight = height;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    // 创建DIB Section
    let mut bits: *mut winapi::ctypes::c_void = ptr::null_mut();
    let h_bitmap = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);

    if h_bitmap.is_null() {
        SelectObject(hdc_mem, hbm_old);
        DeleteDC(hdc_mem);
        ReleaseDC(ptr::null_mut(), hdc_screen);
        if !icon_info.hbmColor.is_null() {
            DeleteObject(icon_info.hbmColor as *mut _);
        }
        if !icon_info.hbmMask.is_null() {
            DeleteObject(icon_info.hbmMask as *mut _);
        }
        return None;
    }

    // 选择新位图到内存DC
    let hbm_old_dib = SelectObject(hdc_mem, h_bitmap as *mut _);

    // 将原图标绘制到位图上
    DrawIconEx(
        hdc_mem,
        0,
        0,
        h_icon,
        width,
        height,
        0,
        ptr::null_mut(),
        0x0003, // DI_NORMAL
    );

    // 获取DIB位
    let buffer_size = (width * height * 4) as usize;
    let mut buffer: Vec<u8> = vec![0; buffer_size];

    let nbits = GetDIBits(
        hdc_mem,
        h_bitmap,
        0,
        height as u32,
        buffer.as_mut_ptr() as *mut _,
        &mut bmi,
        DIB_RGB_COLORS,
    );

    // 恢复DC状态
    SelectObject(hdc_mem, hbm_old_dib);
    SelectObject(hdc_mem, hbm_old);

    // 清理资源
    DeleteObject(h_bitmap as *mut _);
    DeleteDC(hdc_mem);
    ReleaseDC(ptr::null_mut(), hdc_screen);

    if nbits != 0 {
        // 将BGRA转换为RGBA（Windows使用BGRA，而PNG使用RGBA）
        for chunk in buffer.chunks_exact_mut(4) {
            chunk.swap(0, 2); // 交换B和R通道
        }

        // 由于Windows DIB格式的Y轴方向与标准图像相反，需要垂直翻转图像
        let row_size = (width * 4) as usize;
        let mut flipped_buffer = Vec::with_capacity(buffer.len());

        // 从最后一行开始复制到新缓冲区，实现垂直翻转
        for row in (0..height).rev() {
            let start = (row * width * 4) as usize;
            let end = start + row_size;
            flipped_buffer.extend_from_slice(&buffer[start..end]);
        }

        // 将图标数据转换为PNG格式
        use image::ImageFormat;
        use std::io::Cursor;

        if let Some(img) = image::RgbaImage::from_raw(width as u32, height as u32, flipped_buffer) {
            let mut png_data: Vec<u8> = Vec::new();
            if img.write_to(&mut Cursor::new(&mut png_data), ImageFormat::Png).is_ok() {
                if !icon_info.hbmColor.is_null() {
                    DeleteObject(icon_info.hbmColor as *mut _);
                }
                if !icon_info.hbmMask.is_null() {
                    DeleteObject(icon_info.hbmMask as *mut _);
                }

                return Some(png_data);
            }
        }
    }

    // 清理资源
    if !icon_info.hbmColor.is_null() {
        DeleteObject(icon_info.hbmColor as *mut _);
    }
    if !icon_info.hbmMask.is_null() {
        DeleteObject(icon_info.hbmMask as *mut _);
    }

    None
}

// ==================== macOS 平台实现 ====================

#[cfg(target_os = "macos")]
fn extract_icon_from_exe_macos(exe_path: &str) -> Option<Vec<u8>> {
    // 通过字符串操作直接查找路径中的 .app 部分并构建 Resources 路径
    let app_index = exe_path.rfind(".app/");
    if let Some(index) = app_index {
        // 提取 .app 目录路径
        let app_path = &exe_path[..index + 4]; // 包括 ".app"
        let resources_path = format!("{}/Contents/Resources", app_path);

        if std::path::Path::new(&resources_path).exists() {
            // 在 Resources 目录中查找 .icns 文件（只在第一层）
            let resources_dir = std::path::PathBuf::from(resources_path);
            let icns_path = {
                let mut icns_file_path = None;
                if let Ok(entries) = std::fs::read_dir(&resources_dir) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let file_name = entry.file_name();
                            if let Some(name_str) = file_name.to_str() {
                                if name_str.to_lowercase().ends_with(".icns") {
                                    icns_file_path = Some(entry.path());
                                    break; // 只取第一个找到的 .icns 文件
                                }
                            }
                        }
                    }
                }
                icns_file_path?
            };

            // 转换ICNS到PNG
            return convert_icns_to_png(icns_path.to_str()?).ok();
        }
    }

    // 缓存找不到图标的结果
    let cache_key = format!("{:x}", md5::compute(exe_path.as_bytes()));
    let mut cache = ICON_CACHE.lock().unwrap();
    cache.insert(cache_key, (None, SystemTime::now(), false));

    None
}

// macOS辅助函数：将ICNS转换为PNG
#[cfg(target_os = "macos")]
fn convert_icns_to_png(icns_path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::process::Command;
    use tempfile::NamedTempFile;

    // 创建临时文件用于输出PNG
    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path().to_str().ok_or("Invalid temp file path")?;

    eprintln!("Converting ICNS to PNG: {} -> {}", icns_path, temp_path);

    // 使用sips命令将ICNS转换为PNG
    // 注意：sips命令参数顺序很重要，源文件应该在最后
    let output = Command::new("sips")
        .args(&[
            "-s", "format", "png",     // 设置输出格式为PNG
            icns_path, // 输入文件
            "-o", temp_path, // 输出文件
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "sips command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
            .into());
    }

    // 读取临时文件内容
    let png_data = std::fs::read(temp_path)?;

    eprintln!("Conversion successful, PNG size: {} bytes", png_data.len());

    if png_data.is_empty() {
        return Err("Converted PNG data is empty".into());
    }

    Ok(png_data)
}

// ==================== Linux 平台实现 ====================

#[cfg(target_os = "linux")]
fn extract_icon_from_exe_linux(exe_path: &str) -> Option<Vec<u8>> {
    use std::path::Path;

    // 从可执行文件路径提取进程名
    let basename = Path::new(exe_path)
        .file_name()?
        .to_string_lossy()
        .to_string();

    // 尝试多种可能的图标路径
    let icon_sizes = [
        "16x16", "24x24", "32x32", "48x48", "64x64", "128x128", "256x256", "scalable",
    ];
    let icon_themes = ["hicolor", "oxygen", "gnome", "breeze"];
    let icon_types = ["apps", "categories", "devices", "mimetypes"];

    // 首先尝试桌面文件中指定的图标
    let desktop_file_path = format!("/usr/share/applications/{}.desktop", &basename);
    if Path::new(&desktop_file_path).exists() {
        if let Ok(desktop_content) = std::fs::read_to_string(&desktop_file_path) {
            for line in desktop_content.lines() {
                if line.starts_with("Icon=") {
                    let icon_name = line.strip_prefix("Icon=").unwrap_or("");
                    if !icon_name.is_empty() {
                        // 尝试查找该图标名称
                        for size in &icon_sizes {
                            for theme in &icon_themes {
                                for icon_type in &icon_types {
                                    let icon_path = format!(
                                        "/usr/share/icons/{}/{}/{}/{}.png",
                                        theme, size, icon_type, icon_name
                                    );
                                    if Path::new(&icon_path).exists() {
                                        if let Ok(image_data) = std::fs::read(&icon_path) {
                                            return Some(image_data);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 如果桌面文件没有帮助，尝试基于进程名查找图标
    for size in &icon_sizes {
        for theme in &icon_themes {
            for icon_type in &icon_types {
                let icon_paths = vec![
                    format!(
                        "/usr/share/icons/{}/{}/{}/{}.png",
                        theme, size, icon_type, &basename
                    ),
                    format!(
                        "/usr/share/icons/{}/{}/{}/{}.svg",
                        theme, size, icon_type, &basename
                    ),
                    format!("/usr/share/pixmaps/{}.png", &basename),
                    format!("/usr/share/pixmaps/{}.svg", &basename),
                ];

                for icon_path in icon_paths {
                    if Path::new(&icon_path).exists() {
                        if let Ok(image_data) = std::fs::read(&icon_path) {
                            return Some(image_data);
                        }
                    }
                }
            }
        }
    }

    // 缓存找不到图标的结果
    let cache_key = format!("{:x}", md5::compute(exe_path.as_bytes()));
    let mut cache = ICON_CACHE.lock().unwrap();
    cache.insert(cache_key, (None, SystemTime::now(), false));

    None
}