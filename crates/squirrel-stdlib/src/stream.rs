//! ↔ sqstdlib/sqstdstream.cpp + sqstdblobimpl.h + sqstdio.cpp(file 部分)
//!
//! stream / blob / file 三件套：
//! - `stream`：基类（readblob/readn/writeblob/writen/seek/tell/len/eos/flush/_cloned）
//! - `blob`：内存缓冲区（resize/swap2/swap4/_get/_set/_nexti/_cloned）
//! - `file`：文件句柄（constructor/close）
//!
//! C++ 用 sq_setinstanceup 把 SQStream* 挂在实例上；Rust 侧数据存线程本地
//! 注册表，实例字段 `__sid`（实习字符串键）保存注册表索引。

use std::cell::RefCell;
use std::io::{Read, Seek, SeekFrom, Write};
use std::rc::Rc;

use squirrel::class::SQClass;
use squirrel::config::*;
use squirrel::object::{ClassRef, Value};
use squirrel::state::SharedState;
use squirrel::vm::VmData;

// ---------------------------------------------------------------------------
// 数据侧：流后端
// ---------------------------------------------------------------------------

pub const SQ_SEEK_SET: SQInteger = 0;
pub const SQ_SEEK_CUR: SQInteger = 1;
pub const SQ_SEEK_END: SQInteger = 2;

/// ↔ SQStream 的 Rust 等价物（blob 与 file 的共同接口）
pub trait StreamBackend {
    fn read(&mut self, buf: &mut [u8]) -> SQInteger;
    fn write(&mut self, buf: &[u8]) -> SQInteger;
    fn flush(&mut self) -> SQInteger;
    fn tell(&mut self) -> SQInteger;
    fn len(&mut self) -> SQInteger;
    fn seek(&mut self, offset: SQInteger, origin: SQInteger) -> SQInteger;
    fn is_valid(&self) -> bool;
    /// `len()` 是否可信：不可 seek 的流（管道/控制台）上参考实现的 `Len()` 是
    /// `fseek(SEEK_END)` 失败后的 CRT 内部状态（实测同一进程里先返回 4、后返回 0），
    /// 属不可复刻的 UB。`_stream_readblob` 会用它把请求长度夹到 `Len()`，
    /// 这里必须知道「能不能信」，否则管道上会夹成 0 → 永远报 no data left to read。
    fn len_is_valid(&self) -> bool {
        true
    }
    /// ↔ SQBlob::Resize（file 返回 false）
    fn resize(&mut self, _n: SQInteger) -> bool {
        false
    }
    /// ↔ SQFile::Close
    fn close(&mut self) {}
    /// 整块快照（swap2/swap4/_cloned 用；会移动读写位置）
    fn snapshot(&mut self) -> Vec<u8> {
        let len = self.len().max(0);
        let mut out = vec![0u8; len as usize];
        let prev = self.tell();
        if len > 0 {
            self.seek(0, SQ_SEEK_SET);
            self.read(&mut out);
        }
        self.seek(prev, SQ_SEEK_SET);
        out
    }
    fn eos(&mut self) -> bool {
        self.tell() == self.len()
    }
}

/// ↔ SQBlob（sqstdblobimpl.h）
pub struct BlobBackend {
    buf: Vec<u8>,
    size: SQInteger,
    ptr: SQInteger,
}

impl BlobBackend {
    pub fn new(size: SQInteger) -> BlobBackend {
        let n = size.max(0) as usize;
        BlobBackend {
            buf: vec![0u8; n],
            size,
            ptr: 0,
        }
    }
    fn can_advance(&self, n: SQInteger) -> bool {
        self.ptr + n <= self.size
    }
    fn grow_buf_of(&mut self, n: SQInteger) {
        if self.size + n > self.buf.len() as SQInteger {
            if self.size + n > self.size * 2 {
                self.resize_inner(self.size + n);
            } else {
                self.resize_inner(self.size * 2);
            }
        }
        self.size += n;
    }
    /// ↔ SQBlob::Resize：新缓冲清零、保留旧数据前缀、修正 size/ptr
    fn resize_inner(&mut self, n: SQInteger) {
        if n < 0 {
            return;
        }
        let n = n as usize;
        if n != self.buf.len() {
            self.buf.resize(n, 0);
            if self.size > n as SQInteger {
                self.size = n as SQInteger;
            }
            if self.ptr > n as SQInteger {
                self.ptr = n as SQInteger;
            }
        }
    }
}

impl StreamBackend for BlobBackend {
    fn read(&mut self, out: &mut [u8]) -> SQInteger {
        let size = out.len() as SQInteger;
        let mut n = size;
        if !self.can_advance(size) {
            if self.size - self.ptr > 0 {
                n = self.size - self.ptr;
            } else {
                return 0;
            }
        }
        out[..n as usize].copy_from_slice(&self.buf[self.ptr as usize..(self.ptr + n) as usize]);
        self.ptr += n;
        n
    }
    fn write(&mut self, data: &[u8]) -> SQInteger {
        let size = data.len() as SQInteger;
        if !self.can_advance(size) {
            self.grow_buf_of(self.ptr + size - self.size);
        }
        let start = self.ptr as usize;
        let end = start + data.len();
        if end > self.buf.len() {
            self.buf.resize(end, 0);
        }
        self.buf[start..end].copy_from_slice(data);
        self.ptr += size;
        size
    }
    fn flush(&mut self) -> SQInteger {
        0
    }
    fn tell(&mut self) -> SQInteger {
        self.ptr
    }
    fn len(&mut self) -> SQInteger {
        self.size
    }
    fn seek(&mut self, offset: SQInteger, origin: SQInteger) -> SQInteger {
        match origin {
            SQ_SEEK_SET => {
                if offset > self.size || offset < 0 {
                    return -1;
                }
                self.ptr = offset;
            }
            SQ_SEEK_CUR => {
                if self.ptr + offset > self.size || self.ptr + offset < 0 {
                    return -1;
                }
                self.ptr += offset;
            }
            SQ_SEEK_END => {
                if offset > 0 || self.size + offset < 0 {
                    return -1;
                }
                self.ptr = self.size + offset;
            }
            _ => return -1,
        }
        0
    }
    fn is_valid(&self) -> bool {
        self.size == 0 || !self.buf.is_empty()
    }
    fn resize(&mut self, n: SQInteger) -> bool {
        self.resize_inner(n);
        true
    }
    fn snapshot(&mut self) -> Vec<u8> {
        self.buf[..self.size.max(0) as usize].to_vec()
    }
    fn close(&mut self) {}
}

/// ↔ SQFile（sqstdio.cpp:63）
pub struct FileBackend {
    handle: Option<Rc<RefCell<std::fs::File>>>,
    /// ↔ stdin/stdout/stderr 句柄
    std_fd: Option<u8>,
    /// 逻辑读写位置（↔ CRT 的 ftell：管道上就是已消费的字节数）
    pos: SQInteger,
    /// 标准流是否为可 seek 的磁盘文件（重定向自文件时为真）
    std_disk_size: Option<SQInteger>,
}

/// 标准流句柄信息：是否是磁盘文件、文件多大
///
/// 只用 GetFileSizeEx 查询，**不** seek——参考实现的 `Len()` 靠 `fseek(SEEK_END)`，
/// 在管道上会把 stdio 状态搞坏；磁盘文件上两者结果相同（都是文件总长度）。
#[cfg(windows)]
fn std_handle_info(fd: u8) -> (bool, Option<SQInteger>) {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(nstdhandle: u32) -> *mut core::ffi::c_void;
        fn GetFileType(hfile: *mut core::ffi::c_void) -> u32;
        fn GetFileSizeEx(hfile: *mut core::ffi::c_void, filesize: *mut i64) -> i32;
    }
    const FILE_TYPE_DISK: u32 = 1;
    let n = match fd {
        0 => -10i32, // STD_INPUT_HANDLE
        1 => -11i32, // STD_OUTPUT_HANDLE
        _ => -12i32, // STD_ERROR_HANDLE
    } as u32;
    let h = unsafe { GetStdHandle(n) };
    if h.is_null() {
        return (false, None);
    }
    let is_disk = unsafe { GetFileType(h) } == FILE_TYPE_DISK;
    if !is_disk {
        return (false, None);
    }
    let mut size: i64 = 0;
    if unsafe { GetFileSizeEx(h, &mut size) } != 0 {
        (true, Some(size as SQInteger))
    } else {
        (true, None)
    }
}

#[cfg(not(windows))]
fn std_handle_info(_fd: u8) -> (bool, Option<SQInteger>) {
    (false, None)
}

/// CRT 模式串是否合法（↔ `fopen` 接受的那一档）。
///
/// 第一个字符必须是 `r`/`w`/`a`，其余只允许 `+ b t c n`（同一个字符不能重复；
/// 空格按实测是允许的：参考实现 `file(p,"r b")` 正常，`"r\tb"` 则崩）。
fn mode_is_valid(mode: &str) -> bool {
    let b = mode.as_bytes();
    if b.is_empty() || !matches!(b[0], b'r' | b'w' | b'a') {
        return false;
    }
    let mut seen = [false; 128];
    for &c in &b[1..] {
        match c {
            b' ' => continue,
            b'+' | b'b' | b't' | b'c' | b'n' => {
                if seen[c as usize] {
                    return false;
                }
                seen[c as usize] = true;
            }
            _ => return false,
        }
    }
    true
}

impl FileBackend {
    pub fn from_path(filename: &str, mode: &str) -> Option<FileBackend> {
        // ↔ CRT 模式串校验。参考实现把模式串**直接**交给 `fopen`：非法模式会落到
        // UCRT 的 invalid parameter handler（`__fastfail`），实测
        // `file("x","z")` / `""` / `"R"` / `"rbb"` / `"trash"` 全部 `0xC0000409` 崩溃
        // （STATUS 第四节）。本实现改成**可捕获**的 `cannot open file`。
        if !mode_is_valid(mode) {
            return None;
        }
        let mut opts = std::fs::OpenOptions::new();
        // ↔ CRT 模式串
        let read = mode.contains('r');
        let write = mode.contains('w');
        let append = mode.contains('a');
        let plus = mode.contains('+');
        if read || (!read && !write && !append) {
            opts.read(true).write(plus);
        } else if write {
            if append {
                opts.append(true).read(plus).create(true);
            } else {
                opts.write(true).read(plus).create(true).truncate(true);
            }
        } else if append {
            opts.append(true).read(plus).create(true);
        }
        // 注：参考实现的 fopen 用 _SH_DENYNO（FILE_SHARE_READ|WRITE，**不含**
        // FILE_SHARE_DELETE）→ 句柄开着时 `remove()` 失败；Rust 的 std::fs::File
        // 默认三种共享齐全（开着也能删）。**暂不打开** share_mode：虽然 P14 的注册表
        // 滞留已修（后端随实例死亡释放），但还发现"实例可能在**作用域结束前**就被释放"
        // （证据：函数里 `local a = file(p,"wb+"); return try_remove(p);` 我们给 ok、
        // 参考给 failed；中间加一次 `a.len()` 就两边一致）—— 那是下一个要查的点，
        // 在此之前去掉 share-delete 会让这类脚本行为反而偏离参考实现。
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            const FILE_SHARE_READ: u32 = 0x0000_0001;
            const FILE_SHARE_WRITE: u32 = 0x0000_0002;
            opts.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
        }
        match opts.open(filename) {
            Ok(f) => Some(FileBackend {
                handle: Some(Rc::new(RefCell::new(f))),
                std_fd: None,
                pos: 0,
                std_disk_size: None,
            }),
            Err(_) => None,
        }
    }
    pub fn from_std(fd: u8) -> FileBackend {
        let (_disk, size) = std_handle_info(fd);
        FileBackend {
            handle: None,
            std_fd: Some(fd),
            pos: 0,
            std_disk_size: size,
        }
    }
}

impl StreamBackend for FileBackend {
    fn read(&mut self, out: &mut [u8]) -> SQInteger {
        let n = if let Some(h) = &self.handle {
            match h.borrow_mut().read(out) {
                Ok(n) => n as SQInteger,
                Err(_) => 0,
            }
        } else if self.std_fd == Some(0) {
            // ↔ CRT fread：一直读到填满请求长度或 EOF/出错为止。
            // （早先按字节读、遇到 '\n' 就停下，那是把控制台的行语义错安到了管道上：
            //   实测管道里喂 "A\nBC" 后 readblob(4)，参考实现返回 4 字节，不是 2。）
            let stdin = std::io::stdin();
            let mut lock = stdin.lock();
            let mut n = 0usize;
            while n < out.len() {
                match lock.read(&mut out[n..]) {
                    Ok(0) => break,
                    Ok(k) => n += k,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            n as SQInteger
        } else {
            0
        };
        self.pos += n;
        n
    }
    fn write(&mut self, data: &[u8]) -> SQInteger {
        let n = if let Some(h) = &self.handle {
            match h.borrow_mut().write(data) {
                Ok(n) => n as SQInteger,
                Err(_) => 0,
            }
        } else if self.std_fd.is_some() {
            let r = match self.std_fd {
                Some(1) => std::io::stdout().write(data),
                Some(2) => std::io::stderr().write(data),
                _ => Ok(0),
            };
            match r {
                Ok(n) => n as SQInteger,
                Err(_) => 0,
            }
        } else {
            0
        };
        self.pos += n;
        n
    }
    fn flush(&mut self) -> SQInteger {
        if let Some(h) = &self.handle {
            let _ = h.borrow_mut().flush();
        } else {
            let _ = std::io::stdout().flush();
        }
        0
    }
    fn tell(&mut self) -> SQInteger {
        // ↔ sqstd_ftell：文件上是真实偏移，管道上就是已消费的字节数（实测一致）
        if self.handle.is_some() {
            if let Some(h) = &self.handle {
                if let Ok(p) = h.borrow_mut().stream_position() {
                    self.pos = p as SQInteger;
                }
            }
        }
        self.pos
    }
    fn len(&mut self) -> SQInteger {
        if let Some(h) = &self.handle {
            let mut f = h.borrow_mut();
            let prev = f.stream_position().unwrap_or(0) as SQInteger;
            let _ = f.seek(SeekFrom::End(0));
            let size = f.stream_position().unwrap_or(0) as SQInteger;
            let _ = f.seek(SeekFrom::Start(prev.max(0) as u64));
            self.pos = prev;
            size
        } else {
            // 标准流：重定向自文件时是文件总长度；管道/控制台上参考实现的 Len() 是
            // fseek(SEEK_END) 失败后的 CRT 内部状态（不可复刻），这里给 0
            self.std_disk_size.unwrap_or(0)
        }
    }
    fn len_is_valid(&self) -> bool {
        self.handle.is_some() || self.std_disk_size.is_some()
    }
    fn seek(&mut self, offset: SQInteger, origin: SQInteger) -> SQInteger {
        if let Some(h) = &self.handle {
            let pos = match origin {
                // CRT 的 fseek(f, 负数, SEEK_SET) 会失败（实测参考实现返回 -1）；
                // 原来这里用 offset.max(0) 夹成 0，于是"负绝对偏移"被当成成功。
                SQ_SEEK_SET => {
                    if offset < 0 {
                        return -1;
                    }
                    SeekFrom::Start(offset as u64)
                }
                SQ_SEEK_CUR => SeekFrom::Current(offset),
                SQ_SEEK_END => SeekFrom::End(offset),
                _ => return -1,
            };
            let mut f = h.borrow_mut();
            match f.seek(pos) {
                Ok(p) => {
                    self.pos = p as SQInteger;
                    0
                }
                Err(_) => -1,
            }
        } else {
            -1
        }
    }
    fn is_valid(&self) -> bool {
        self.handle.is_some() || self.std_fd.is_some()
    }
    fn close(&mut self) {
        if let Some(h) = &self.handle {
            let _ = h.borrow_mut().flush();
        }
        self.handle = None;
        self.std_fd = None;
    }
}

// ---------------------------------------------------------------------------
// 注册表：实例 <-> 后端数据
// ---------------------------------------------------------------------------

pub type BackendRef = Rc<RefCell<dyn StreamBackend>>;

thread_local! {
    static BACKENDS: RefCell<Vec<Option<BackendRef>>> = const { RefCell::new(Vec::new()) };
}

pub fn register_backend(b: BackendRef) -> SQInteger {
    BACKENDS.with(|reg| {
        let mut r = reg.borrow_mut();
        r.push(Some(b));
        (r.len() - 1) as SQInteger
    })
}

/// 从实例（栈槽 idx）取后端（键为实习字符串 __sid）
pub fn backend_of(v: &VmData, idx: SQInteger) -> Option<BackendRef> {
    let o = v.stack_get(idx);
    let sid_key = v.ss.intern(b"__sid");
    if let Value::Instance(inst) = &o {
        let sid = inst.borrow().get(&sid_key)?;
        let id = sid.as_integer();
        return BACKENDS.with(|reg| reg.borrow().get(id as usize).and_then(|x| x.clone()));
    }
    None
}

/// 在实例（栈槽 1）上设置 __sid 字段
fn set_instance_sid(v: &VmData, sid: SQInteger) {
    let self_ = v.stack_get(1);
    let sid_key = v.ss.intern(b"__sid");
    if let Value::Instance(inst) = &self_ {
        inst.borrow_mut().set(&sid_key, &Value::Integer(sid));
    }
}

/// 释放注册表的某个槽位（只在确实还是同一个后端时才清，避免跨线程 sid 碰撞误清）。
fn release_backend_slot(sid: SQInteger, expect: usize) {
    BACKENDS.with(|reg| {
        let mut r = reg.borrow_mut();
        if let Some(slot) = r.get_mut(sid as usize) {
            let same = slot
                .as_ref()
                .map(|b| Rc::as_ptr(b) as *const () as usize == expect)
                .unwrap_or(false);
            if same {
                *slot = None;
            }
        }
    });
}

/// 把后端绑到实例上：注册表存**强引用**，实例侧再挂一个 userdata 成员 `__be`，
/// 它的 release hook 在**实例死亡时**清掉注册表槽位 → 强引用释放 → 后端（连同它持有的
/// 文件句柄）立即析构。
///
/// 为什么要这样：注册表原来只 push、从不清理，于是**每个创建过的 file/blob/stream 后端
/// 都被永久持有**（无界增长；没显式 `close()` 的文件句柄会一直开到进程结束）。参考实现是
/// 引用计数归零就释放，所以同一段脚本里"别人删得掉的文件"我们必须也删得掉
/// （见 TODO.md P14）。
fn bind_backend_inst(
    v: &mut VmData,
    inst: &squirrel::object::InstanceRef,
    backend: BackendRef,
) -> SQInteger {
    let sid = register_backend(backend.clone());
    // 注意：`Rc<RefCell<dyn Trait>>` 的 as_ptr 是胖指针，先转成薄指针再取整数
    let expect = Rc::as_ptr(&backend) as *const () as usize;
    let ud = squirrel::userdata::SQUserData::create(&v.ss, 0);
    {
        let mut u = ud.borrow_mut();
        u.hook = Some(Box::new(move |_p, _s| {
            release_backend_slot(sid, expect);
            0
        }));
    }
    inst.borrow_mut()
        .set(&v.ss.intern(b"__be"), &Value::Userdata(ud));
    inst.borrow_mut()
        .set(&v.ss.intern(b"__sid"), &Value::Integer(sid));
    sid
}

/// 同 `bind_backend_inst`，但作用于栈槽 1 的 `this`（blob/file 构造器与 `_cloned` 用）
fn bind_backend_this(v: &mut VmData, backend: BackendRef) -> SQInteger {
    let inst = match v.stack_get(1) {
        Value::Instance(i) => i,
        _ => return 0,
    };
    bind_backend_inst(v, &inst, backend)
}

fn stream_err(v: &mut VmData, msg: &str) -> SQInteger {
    v.raise_error_str(msg);
    SQ_ERROR
}

// ---------------------------------------------------------------------------
// 类构建
// ---------------------------------------------------------------------------

fn method(
    ss: &Rc<SharedState>,
    cls: &ClassRef,
    name: &str,
    f: squirrel::closure::NativeFn,
    npc: SQInteger,
    tm: Option<&str>,
) {
    let nc = squirrel::closure::SQNativeClosure::create(ss, f, 0);
    {
        let mut n = nc.borrow_mut();
        n.nparamscheck = npc;
        n.name = ss.intern(name.as_bytes());
        if let Some(t) = tm {
            squirrel::state::compile_typemask(&mut n.typecheck, t.as_bytes());
        }
    }
    let key = ss.intern(name.as_bytes());
    cls.borrow_mut().new_slot(ss, &key, &Value::NativeClosure(nc), true);
}

/// ↔ init_streamclass：stream 基类
fn make_stream_class(ss: &Rc<SharedState>) -> ClassRef {
    let cls = SQClass::create(ss, None);
    // ↔ _stream_methods（sqstdstream.cpp:241）
    method(ss, &cls, "readblob", Rc::new(stream_readblob), 2, Some("xn"));
    method(ss, &cls, "readn", Rc::new(stream_readn), 2, Some("xn"));
    method(ss, &cls, "writeblob", Rc::new(stream_writeblob), -2, Some("xx"));
    method(ss, &cls, "writen", Rc::new(stream_writen), 3, Some("xnn"));
    method(ss, &cls, "seek", Rc::new(stream_seek), -2, Some("xnn"));
    method(ss, &cls, "tell", Rc::new(stream_tell), 1, Some("x"));
    method(ss, &cls, "len", Rc::new(stream_len), 1, Some("x"));
    method(ss, &cls, "eos", Rc::new(stream_eos), 1, Some("x"));
    method(ss, &cls, "flush", Rc::new(stream_flush), 1, Some("x"));
    method(ss, &cls, "_cloned", Rc::new(|v: &mut VmData| {
        v.raise_error_str("this object cannot be cloned");
        SQ_ERROR
    }), 0, None);
    cls
}

/// ↔ _file_methods
pub fn make_file_class(ss: &Rc<SharedState>, stream_cls: &ClassRef) -> ClassRef {
    let cls = SQClass::create(ss, Some(stream_cls.clone()));
    // 实例字段 __sid（后端注册表索引）与 __be（持有后端强引用的 userdata，见 bind_backend_inst）。
    // 必须**声明成类字段**：SQInstance::set 对未声明的键直接返回 false（Squirrel 语义），
    // 不声明的话 userdata 只会被临时引用持有、立刻析构 —— 钩子会提前清掉注册表槽位，
    // 于是刚建出来的 blob/file 都变成 "the stream is invalid"（踩过）。
    cls.borrow_mut().new_slot(ss, &ss.intern(b"__sid"), &Value::Integer(0), false);
    cls.borrow_mut().new_slot(ss, &ss.intern(b"__be"), &Value::Null, false);
    method(ss, &cls, "constructor", Rc::new(file_constructor), 3, Some("x"));
    method(ss, &cls, "_typeof", Rc::new(|v: &mut VmData| {
        v.push(v.ss.intern(b"file"));
        1
    }), 1, Some("x"));
    method(ss, &cls, "close", Rc::new(file_close), 1, Some("x"));
    cls
}

/// ↔ _blob_methods
pub fn make_blob_class(ss: &Rc<SharedState>, stream_cls: &ClassRef) -> ClassRef {
    let cls = SQClass::create(ss, Some(stream_cls.clone()));
    // 实例字段 __sid 与 __be（见 make_file_class 里的说明：必须声明成类字段）
    cls.borrow_mut().new_slot(ss, &ss.intern(b"__sid"), &Value::Integer(0), false);
    cls.borrow_mut().new_slot(ss, &ss.intern(b"__be"), &Value::Null, false);
    method(ss, &cls, "constructor", Rc::new(blob_constructor), -1, Some("xn"));
    method(ss, &cls, "resize", Rc::new(blob_resize), 2, Some("xn"));
    method(ss, &cls, "swap2", Rc::new(blob_swap2), 1, Some("x"));
    method(ss, &cls, "swap4", Rc::new(blob_swap4), 1, Some("x"));
    method(ss, &cls, "_set", Rc::new(blob_set), 3, Some("xnn"));
    method(ss, &cls, "_get", Rc::new(blob_get), 2, Some("x."));
    method(ss, &cls, "_typeof", Rc::new(|v: &mut VmData| {
        v.push(v.ss.intern(b"blob"));
        1
    }), 1, Some("x"));
    method(ss, &cls, "_nexti", Rc::new(blob_nexti), 2, Some("x"));
    method(ss, &cls, "_cloned", Rc::new(blob_cloned), 2, Some("xx"));
    cls
}

// ---------------------------------------------------------------------------
// stream 方法（↔ sqstdstream.cpp）
// ---------------------------------------------------------------------------

/// ↔ _stream_readblob
fn stream_readblob(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) if b.borrow().is_valid() => b,
        _ => return stream_err(v, "the stream is invalid"),
    };
    let mut size = v.stack_get(2).as_integer();
    // ↔ _stream_readblob 的 `if(size > self->Len()) size = self->Len();`
    // 那只是 scratchpad 的尺寸估计；不可 seek 的流上 Len() 不可信（管道上是 UB 值），
    // 照夹会把请求夹成 0 → 管道上永远 "no data left to read"
    let (len, valid) = {
        let mut b = self_.borrow_mut();
        (b.len(), b.len_is_valid())
    };
    if valid && size > len {
        size = len;
    }
    if size < 0 {
        size = 0;
    }
    let mut data = vec![0u8; size as usize];
    let res = self_.borrow_mut().read(&mut data);
    if res <= 0 {
        return stream_err(v, "no data left to read");
    }
    data.truncate(res as usize);
    push_blob_instance(v, data)
}

/// ↔ _stream_readn（格式字符 l/i/s/w/c/b/f/d）
fn stream_readn(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) if b.borrow().is_valid() => b,
        _ => return stream_err(v, "the stream is invalid"),
    };
    let format = v.stack_get(2).as_integer();
    macro_rules! rd {
        ($n:expr) => {{
            let mut buf = [0u8; $n];
            if self_.borrow_mut().read(&mut buf) != $n as SQInteger {
                return stream_err(v, "io error");
            }
            buf
        }};
    }
    match format as u8 {
        b'l' => {
            let b = rd!(8);
            v.push(Value::Integer(SQInteger::from_le_bytes(b)));
        }
        b'i' => {
            let b = rd!(4);
            v.push(Value::Integer(i32::from_le_bytes(b) as SQInteger));
        }
        b's' => {
            let b = rd!(2);
            v.push(Value::Integer(i16::from_le_bytes(b) as SQInteger));
        }
        b'w' => {
            let b = rd!(2);
            v.push(Value::Integer(u16::from_le_bytes(b) as SQInteger));
        }
        b'c' => {
            let b = rd!(1);
            v.push(Value::Integer(b[0] as i8 as SQInteger));
        }
        b'b' => {
            let b = rd!(1);
            v.push(Value::Integer(b[0] as SQInteger));
        }
        b'f' => {
            let b = rd!(4);
            v.push(Value::Float(f32::from_le_bytes(b)));
        }
        b'd' => {
            let b = rd!(8);
            v.push(Value::Float(f64::from_le_bytes(b) as SQFloat));
        }
        _ => return stream_err(v, "invalid format"),
    }
    1
}

/// ↔ _stream_writeblob
fn stream_writeblob(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) if b.borrow().is_valid() => b,
        _ => return stream_err(v, "the stream is invalid"),
    };
    let blob = v.stack_get(2);
    let data = match get_blob_bytes(v, &blob) {
        Some(d) => d,
        None => return stream_err(v, "invalid parameter"),
    };
    let size = data.len() as SQInteger;
    if self_.borrow_mut().write(&data) != size {
        return stream_err(v, "io error");
    }
    v.push(Value::Integer(size));
    1
}

/// ↔ _stream_writen
fn stream_writen(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) if b.borrow().is_valid() => b,
        _ => return stream_err(v, "the stream is invalid"),
    };
    let val = v.stack_get(2);
    let format = v.stack_get(3).as_integer();
    let bytes: Vec<u8> = match format as u8 {
        b'l' => val.as_integer().to_le_bytes().to_vec(),
        b'i' => (val.as_integer() as i32).to_le_bytes().to_vec(),
        b's' => (val.as_integer() as i16).to_le_bytes().to_vec(),
        b'w' => (val.as_integer() as u16).to_le_bytes().to_vec(),
        b'c' => vec![val.as_integer() as i8 as u8],
        b'b' => vec![val.as_integer() as u8],
        b'f' => val.as_float().to_le_bytes().to_vec(),
        b'd' => (val.as_float() as f64).to_le_bytes().to_vec(),
        _ => return stream_err(v, "invalid format"),
    };
    self_.borrow_mut().write(&bytes);
    0
}

/// ↔ _stream_seek
fn stream_seek(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) if b.borrow().is_valid() => b,
        _ => return stream_err(v, "the stream is invalid"),
    };
    let offset = v.stack_get(2).as_integer();
    let mut origin = SQ_SEEK_SET;
    if v.top - v.stackbase > 2 {
        match v.stack_get(3).as_integer() as u8 {
            b'b' => origin = SQ_SEEK_SET,
            b'c' => origin = SQ_SEEK_CUR,
            b'e' => origin = SQ_SEEK_END,
            _ => return stream_err(v, "invalid origin"),
        }
    }
    let r = self_.borrow_mut().seek(offset, origin);
    v.push(Value::Integer(r));
    1
}

/// ↔ _stream_tell
fn stream_tell(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) if b.borrow().is_valid() => b,
        _ => return stream_err(v, "the stream is invalid"),
    };
    let t = self_.borrow_mut().tell();
    v.push(Value::Integer(t));
    1
}

/// ↔ _stream_len
fn stream_len(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) if b.borrow().is_valid() => b,
        _ => return stream_err(v, "the stream is invalid"),
    };
    let l = self_.borrow_mut().len();
    v.push(Value::Integer(l));
    1
}

/// ↔ _stream_eos：真返回 1，假返回 null
fn stream_eos(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) if b.borrow().is_valid() => b,
        _ => return stream_err(v, "the stream is invalid"),
    };
    if self_.borrow_mut().eos() {
        v.push(Value::Integer(1));
    } else {
        v.push_null();
    }
    1
}

/// ↔ _stream_flush：成功返回 1，失败返回 null
fn stream_flush(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) if b.borrow().is_valid() => b,
        _ => return stream_err(v, "the stream is invalid"),
    };
    if self_.borrow_mut().flush() == 0 {
        v.push(Value::Integer(1));
    } else {
        v.push_null();
    }
    1
}

// ---------------------------------------------------------------------------
// blob 方法（↔ sqstdblob.cpp）
// ---------------------------------------------------------------------------

/// ↔ _blob_constructor
fn blob_constructor(v: &mut VmData) -> SQInteger {
    let nparam = v.top - v.stackbase;
    let mut size: SQInteger = 0;
    if nparam == 2 {
        size = v.stack_get(2).as_integer();
    }
    if size < 0 {
        return stream_err(v, "cannot create blob with negative size");
    }
    let backend: BackendRef = Rc::new(RefCell::new(BlobBackend::new(size)));
    bind_backend_this(v, backend);
    0
}

/// ↔ _blob_resize
fn blob_resize(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) => b,
        None => return stream_err(v, "invalid type tag"),
    };
    let size = v.stack_get(2).as_integer();
    if !self_.borrow_mut().resize(size) {
        return stream_err(v, "resize failed");
    }
    0
}

/// ↔ _blob_swap2：按 16 位字翻转字节序
///
/// 参考实现直接对着 `self->GetBuf()` 整块交换，**完全不碰 `_ptr`**（`tell()` 不变）。
/// 我们原来是 snapshot → 改 → `seek(0)` + `write(&buf)`，`write` 会把 `_ptr` 推到末尾，
/// 于是 `tell()` 变成 4/8（模糊测试用 `blob.swap2()` 之后 `tell()` 抓到的差异）。
fn blob_swap2(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) => b,
        None => return stream_err(v, "invalid type tag"),
    };
    let mut be = self_.borrow_mut();
    let len = be.len();
    let num = ((len - len % 2) >> 1).max(0) as usize;
    let pos = be.tell();
    let mut buf = be.snapshot();
    for i in 0..num {
        buf.swap(i * 2, i * 2 + 1);
    }
    be.seek(0, SQ_SEEK_SET);
    be.write(&buf);
    be.seek(pos, SQ_SEEK_SET);
    0
}

/// ↔ _blob_swap4：按 32 位双字翻转字节序（同样不动 `_ptr`）
fn blob_swap4(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) => b,
        None => return stream_err(v, "invalid type tag"),
    };
    let mut be = self_.borrow_mut();
    let len = be.len();
    let num = ((len - len % 4) >> 2).max(0) as usize;
    let pos = be.tell();
    let mut buf = be.snapshot();
    for i in 0..num {
        let b = i * 4;
        buf.swap(b, b + 3);
        buf.swap(b + 1, b + 2);
    }
    be.seek(0, SQ_SEEK_SET);
    be.write(&buf);
    be.seek(pos, SQ_SEEK_SET);
    0
}

/// ↔ _blob__set：b[i] = v
fn blob_set(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) => b,
        None => return stream_err(v, "invalid type tag"),
    };
    let idx = v.stack_get(2).as_integer();
    let val = v.stack_get(3).as_integer();
    let mut be = self_.borrow_mut();
    if idx < 0 || idx >= be.len() {
        return stream_err(v, "index out of range");
    }
    let prev = be.tell();
    be.seek(idx, SQ_SEEK_SET);
    be.write(&[val as u8]);
    be.seek(prev, SQ_SEEK_SET);
    let v3 = v.stack_get(3);
    v.push(v3);
    1
}

/// ↔ _blob__get：b[i]
fn blob_get(v: &mut VmData) -> SQInteger {
    let key = v.stack_get(2);
    if key.type_() & SQOBJECT_NUMERIC == 0 {
        // ↔ 原版：pushnull + sq_throwobject —— 抛出 null（= 干净失败，
        //   让 FallBackGet 继续走默认委托链）
        v.raise_error_obj(Value::Null);
        return SQ_ERROR;
    }
    let self_ = match backend_of(v, 1) {
        Some(b) => b,
        None => return stream_err(v, "invalid type tag"),
    };
    let idx = key.as_integer();
    let mut be = self_.borrow_mut();
    if idx < 0 || idx >= be.len() {
        return stream_err(v, "index out of range");
    }
    let prev = be.tell();
    be.seek(idx, SQ_SEEK_SET);
    let mut b = [0u8; 1];
    be.read(&mut b);
    be.seek(prev, SQ_SEEK_SET);
    v.push(Value::Integer(b[0] as SQInteger));
    1
}

/// ↔ _blob__nexti
fn blob_nexti(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) => b,
        None => return stream_err(v, "invalid type tag"),
    };
    let previdx = v.stack_get(2);
    if previdx.is_null() {
        v.push(Value::Integer(0));
        return 1;
    }
    if let Value::Integer(idx) = &previdx {
        let len = self_.borrow_mut().len();
        if idx + 1 < len {
            v.push(Value::Integer(idx + 1));
        } else {
            v.push_null();
        }
        return 1;
    }
    stream_err(v, "internal error (_nexti) wrong argument type")
}

/// ↔ _blob__cloned
fn blob_cloned(v: &mut VmData) -> SQInteger {
    let other = match backend_of(v, 2) {
        Some(b) => b,
        None => return SQ_ERROR,
    };
    let content = other.borrow_mut().snapshot();
    let len = content.len() as SQInteger;
    let backend: BackendRef = Rc::new(RefCell::new(BlobBackend::new(len)));
    {
        let mut nb = backend.borrow_mut();
        nb.write(&content);
        nb.seek(0, SQ_SEEK_SET);
    }
    bind_backend_this(v, backend);
    0
}

// ---------------------------------------------------------------------------
// file 方法（↔ sqstdio.cpp _file_*）
// ---------------------------------------------------------------------------

/// ↔ _file_constructor：file(path, mode) 或 file(userpointer[, own])
fn file_constructor(v: &mut VmData) -> SQInteger {
    let a = v.stack_get(2);
    let b = v.stack_get(3);
    let backend = match (&a, &b) {
        (Value::String(_), Value::String(_)) => {
            let filename = String::from_utf8_lossy(a.as_bytes()).into_owned();
            let mode = String::from_utf8_lossy(b.as_bytes()).into_owned();
            match FileBackend::from_path(&filename, &mode) {
                Some(f) => f,
                None => return stream_err(v, "cannot open file"),
            }
        }
        (Value::UserPointer(fd), _) => FileBackend::from_std(*fd as u8),
        _ => return stream_err(v, "wrong parameter"),
    };
    let backend: BackendRef = Rc::new(RefCell::new(backend));
    bind_backend_this(v, backend);
    0
}

/// ↔ _file_close
fn file_close(v: &mut VmData) -> SQInteger {
    if let Some(b) = backend_of(v, 1) {
        b.borrow_mut().close();
    }
    0
}

// ---------------------------------------------------------------------------
// blob 工具（供其他模块使用）
// ---------------------------------------------------------------------------

thread_local! {
    static BLOB_CLASS: RefCell<Option<ClassRef>> = const { RefCell::new(None) };
    /// 三个流类只建一次：bloblib 与 iolib 都要用，
    /// 重复构造会让 root 里的类与 BLOB_CLASS 指向不同对象（readblob 立刻报错）
    static STREAM_CLASSES: RefCell<Option<(ClassRef, ClassRef, ClassRef)>> = const { RefCell::new(None) };
}

/// ↔ sqstd_createblob：构造 blob 实例并压栈
pub fn push_blob_instance(v: &mut VmData, data: Vec<u8>) -> SQInteger {
    let backend: BackendRef = Rc::new(RefCell::new(BlobBackend::new(0)));
    {
        let mut b = backend.borrow_mut();
        b.write(&data);
        b.seek(0, SQ_SEEK_SET);
    }
    let blob_cls = BLOB_CLASS.with(|c| c.borrow().clone());
    if let Some(cls) = blob_cls {
        let inst = SQClass::create_instance(&cls, &v.ss);
        bind_backend_inst(v, &inst, backend);
        v.push(Value::Instance(inst));
        1
    } else {
        stream_err(v, "blob class not registered")
    }
}

/// ↔ sqstd_getblob + sqstd_getblobsize：取实例的完整字节内容
pub fn get_blob_bytes(v: &VmData, o: &Value) -> Option<Vec<u8>> {
    let sid_key = v.ss.intern(b"__sid");
    if let Value::Instance(inst) = o {
        let sid = inst.borrow().get(&sid_key)?;
        let id = sid.as_integer();
        let b = BACKENDS.with(|reg| reg.borrow().get(id as usize).and_then(|x| x.clone()))?;
        let mut be = b.borrow_mut();
        let content = be.snapshot();
        return Some(content);
    }
    None
}

/// ↔ sqstd_createfile：为 stdout/stderr/stdin 创建实例并压栈
pub fn push_file_instance(v: &mut VmData, file_cls: &ClassRef, fd: u8) {
    let backend: BackendRef = Rc::new(RefCell::new(FileBackend::from_std(fd)));
    let inst = SQClass::create_instance(file_cls, &v.ss);
    bind_backend_inst(v, &inst, backend);
    v.push(Value::Instance(inst));
}

// ---------------------------------------------------------------------------
// 注册入口
// ---------------------------------------------------------------------------

/// ↔ declare_stream / init_streamclass 的类构造部分（**不**注册进 root）
///
/// 注册顺序必须与参考实现一致（根表迭代顺序可被 `foreach getroottable()` 观察到）：
///     bloblib : stream → casti2f/castf2i/swap2/swap4/swapfloat → blob
///     iolib   : loadfile/dofile/writeclosuretofile → file → stdout → stdin → stderr
/// 所以这里只负责建类，插入 root 的时机交给各 lib 的注册函数。
pub fn make_stream_classes(vm: &mut VmData) -> (ClassRef, ClassRef, ClassRef) {
    let ss = vm.ss.clone();
    // 幂等：第二次调用必须返回同一批类对象（↔ declare_stream 复用 registry 里的 std_stream）
    let cached = STREAM_CLASSES.with(|c| c.borrow().clone());
    if let Some(triple) = cached {
        return triple;
    }
    let stream_cls = make_stream_class(&ss);
    let file_cls = make_file_class(&ss, &stream_cls);
    let blob_cls = make_blob_class(&ss, &stream_cls);
    BLOB_CLASS.with(|c| *c.borrow_mut() = Some(blob_cls.clone()));
    STREAM_CLASSES.with(|c| *c.borrow_mut() = Some((stream_cls.clone(), file_cls.clone(), blob_cls.clone())));
    let _ = vm;
    (stream_cls, file_cls, blob_cls)
}

/// 把一个类注册进 root（↔ declare_stream 末尾的 sq_newslot）
pub fn register_class(vm: &mut VmData, name: &str, cls: &ClassRef) {
    let ss = vm.ss.clone();
    let root = match &vm.roottable {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    root.borrow_mut()
        .new_slot(&ss.intern(name.as_bytes()), &Value::Class(cls.clone()));
}
