//正则表达式判断
pub fn validate(pattern: &str, content: &str) -> bool {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    use regex::Regex;
    #[cfg(any(target_os = "linux"))]
    use tauri::regex::Regex;
    let re = Regex::new(pattern).unwrap();
    re.is_match(content)
}

// 跨平台替换路径最后一部分的函数
pub fn replace_last_path_component(path: &str, new_name: &str) -> String {
    use std::path::Path;

    let path_obj = Path::new(path);
    if let Some(parent) = path_obj.parent() {
        let new_path = parent.join(new_name);
        new_path.to_string_lossy().to_string()
    } else {
        // 如果没有父目录（例如根目录），直接返回新名称
        new_name.to_string()
    }
}

#[test]
fn test() {
    println!("test-- {}", validate(r#"(?:md)$"#, "44.jpg.33.JPEG.md"));
}
