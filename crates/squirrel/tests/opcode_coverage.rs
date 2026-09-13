//! 指令覆盖度：差分语料必须把 61 条指令都跑到，否则那条指令的 VM 语义就没有被对拍过。
//!
//! 用 `cargo test --test opcode_coverage -- --nocapture` 看直方图与缺失清单。
//! 若某条指令确实无法由编译器产生（例如历史遗留编号），把它加进 `UNAVOIDABLE` 并写明原因。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use squirrel::api::Vm;
use squirrel::object::Value;
use squirrel::opcodes::{Op, OP_COUNT};

/// 已确认「编译器不会产生、因此差分语料无法覆盖」的编号（键为编号，值为原因）。
/// 目前为空：61 条都要覆盖。
const UNAVOIDABLE: &[(u8, &str)] = &[];

fn op_name(op: Op) -> String {
    format!("{:?}", op)
}

fn walk(proto: &squirrel::funcproto::SQFunctionProto, hist: &mut BTreeMap<u8, usize>) {
    for ins in &proto.instructions {
        *hist.entry(ins.op).or_insert(0) += 1;
    }
    for f in &proto.functions {
        if let Value::FuncProto(fp) = f {
            walk(fp, hist);
        }
    }
}

/// 编译一个脚本源码，返回 proto（失败返回 None）
fn compile_proto(src: &[u8], name: &[u8]) -> Option<squirrel::object::FuncProtoRef> {
    let vm = Vm::new(1024);
    let mut v = vm.data_mut();
    let bytes = src.to_vec();
    let mut pos = 0usize;
    let readf = Box::new(move || {
        if pos < bytes.len() {
            let c = bytes[pos];
            pos += 1;
            c as i64
        } else {
            0
        }
    });
    // 打开编译错误处理器，便于定位「为什么这个语料文件在这里编译失败」
    {
        let h: squirrel::state::CompilerErrorHandler = std::rc::Rc::new(
            |_v, msg, src, line, col| {
                eprintln!("  编译错误: {} 行 {} 列 {}: {}", src, line, col, msg);
            },
        );
        *v.ss.compilererrorhandler.borrow_mut() = Some(h);
    }
    match squirrel::compiler::compile(&mut v, readf, name, true, true) {
        Ok(Value::FuncProto(f)) => Some(f),
        Ok(other) => {
            eprintln!("{}: 编译返回了非 proto：{:?}", String::from_utf8_lossy(name), other);
            None
        }
        Err(()) => {
            eprintln!(
                "{}: 编译失败：{}",
                String::from_utf8_lossy(name),
                squirrel::api::value_to_error_string(&v.lasterror.clone())
            );
            None
        }
    }
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests-corpus")
}

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../squirrel/samples")
}

fn collect_from_dir(
    dir: &Path,
    hist: &mut BTreeMap<u8, usize>,
    files: &mut Vec<String>,
    skipped: &mut Vec<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "nut").unwrap_or(false))
        .collect();
    paths.sort();
    for p in paths {
        let Ok(mut src) = std::fs::read(&p) else { continue };
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        // 带 UTF-8 BOM 的源文件：CLI 的加载器会跳过 BOM，这里也一样
        if src.starts_with(&[0xEF, 0xBB, 0xBF]) {
            src.drain(0..3);
        }
        match compile_proto(&src, name.as_bytes()) {
            Some(proto) => {
                walk(&proto, hist);
                files.push(name);
            }
            None => skipped.push(name),
        }
    }
}

#[test]
fn every_opcode_is_exercised_by_the_corpus() {
    let mut hist: BTreeMap<u8, usize> = BTreeMap::new();
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    collect_from_dir(&corpus_dir(), &mut hist, &mut files, &mut skipped);
    collect_from_dir(&samples_dir(), &mut hist, &mut files, &mut skipped);

    assert!(!files.is_empty(), "找不到语料目录");
    if !skipped.is_empty() {
        println!("== 编译失败被跳过（{}）==\n{:?}", skipped.len(), skipped);
    }

    let mut missing: Vec<(u8, String)> = Vec::new();
    for code in 0..OP_COUNT {
        if hist.get(&code).copied().unwrap_or(0) == 0 {
            let name = Op::from_u8(code).map(op_name).unwrap_or_else(|| "?".into());
            if !UNAVOIDABLE.iter().any(|(c, _)| *c == code) {
                missing.push((code, name));
            }
        }
    }

    println!("== 指令覆盖（语料 + samples，共 {} 个文件）==", files.len());
    for code in 0..OP_COUNT {
        let name = Op::from_u8(code).map(op_name).unwrap_or_else(|| "?".into());
        let n = hist.get(&code).copied().unwrap_or(0);
        let tag = if n == 0 { "  <== 缺失" } else { "" };
        println!("0x{:02X} {:<12} {:>7}{}", code, name, n, tag);
    }
    let covered = (0..OP_COUNT)
        .filter(|c| hist.get(c).copied().unwrap_or(0) > 0)
        .count();
    println!("覆盖 {}/{}", covered, OP_COUNT);

    assert!(
        missing.is_empty(),
        "以下指令没有被任何语料覆盖：{:?}",
        missing
    );
}
