use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("samples/_/_.txx"));

    let src = fs::read_to_string(&input_path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", input_path.display());
        exit(1);
    });

    // --- 검사 단계: 여기서 걸리면 컴파일 자체를 안 함 ---
    if let Err(errors) = lib::check(&src) {
        for e in &errors {
            eprintln!("{}", e.message);
        }
        eprintln!("\n[txx] {} error(s), compilation aborted", errors.len());
        exit(1);
    }

    let cpp_src = lib::transpile(&src);

    let out_dir = Path::new("target/txx");
    fs::create_dir_all(out_dir).unwrap();

    let stem = input_path.file_stem().unwrap().to_string_lossy();
    let cpp_path = out_dir.join(format!("{stem}.cpp"));
    let bin_path = out_dir.join(stem.as_ref());

    fs::write(&cpp_path, &cpp_src).unwrap();
    println!("[txx] transpiled -> {}", cpp_path.display());

    let compiler = find_compiler();
    println!("[txx] compiling with {compiler}...");

    let status = Command::new(&compiler)
        .arg("-std=c++20")
        .arg(&cpp_path)
        .arg("-o")
        .arg(&bin_path)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("error: failed to run {compiler}: {e}");
            exit(1);
        });

    if !status.success() {
        eprintln!("[txx] compile failed — see {}", cpp_path.display());
        exit(status.code().unwrap_or(1));
    }

    println!("[txx] running {}", bin_path.display());
    let run_status = Command::new(bin_path.canonicalize().unwrap())
        .status()
        .unwrap();
    exit(run_status.code().unwrap_or(0));
}

fn find_compiler() -> String {
    for c in ["clang++", "g++"] {
        if Command::new(c).arg("--version").output().is_ok() {
            return c.to_string();
        }
    }
    eprintln!("error: neither clang++ nor g++ found in PATH");
    exit(1);
}