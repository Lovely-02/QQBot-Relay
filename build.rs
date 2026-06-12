use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

fn main() {
    println!("cargo:rerun-if-changed=webui/package.json");
    println!("cargo:rerun-if-changed=webui/pnpm-lock.yaml");
    println!("cargo:rerun-if-changed=webui/pnpm-workspace.yaml");
    println!("cargo:rerun-if-changed=webui/vite.config.js");
    watch_files(Path::new("webui/src"));

    build_webui();
}

fn watch_files(path: &Path) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            watch_files(&path);
        } else if let Some(path) = path.to_str() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}

fn build_webui() {
    let webui_dir = Path::new("webui");
    if !webui_dir.join("package.json").exists() {
        panic!("未找到 webui/package.json，无法构建 WebUI");
    }

    run_pnpm(
        webui_dir,
        &["install", "--frozen-lockfile", "--prefer-offline"],
        "WebUI 依赖安装失败，请确认 Node.js 和 pnpm 可用",
    );
    run_pnpm(webui_dir, &["build"], "WebUI 构建失败");
}

fn run_pnpm(webui_dir: &Path, args: &[&str], failure_message: &str) {
    let status = pnpm_command()
        .args(args)
        .current_dir(webui_dir)
        .stdin(Stdio::null())
        .status()
        .expect("启动 pnpm 命令失败，请确认已安装 Node.js 和 pnpm");

    if !status.success() {
        panic!("{failure_message}");
    }
}

fn pnpm_command() -> Command {
    if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", "pnpm"]);
        command
    } else {
        Command::new("pnpm")
    }
}
