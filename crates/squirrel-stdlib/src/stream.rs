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
}

impl FileBackend {
    pub fn from_path(filename: &str, mode: &str) -> Option<FileBackend> {
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
        match opts.open(filename) {
            Ok(f) => Some(FileBackend {
                handle: Some(Rc::new(RefCell::new(f))),
                std_fd: None,
            }),
            Err(_) => None,
        }
    }
    pub fn from_std(fd: u8) -> FileBackend {
        FileBackend {
            handle: None,
            std_fd: Some(fd),
        }
    }
}

impl StreamBackend for FileBackend {
    fn read(&mut self, out: &mut [u8]) -> SQInteger {
        if let Some(h) = &self.handle {
            match h.borrow_mut().read(out) {
                Ok(n) => n as SQInteger,
                Err(_) => 0,
            }
        } else if self.std_fd == Some(0) {
            #[allow(unused_imports)]
            use std::io::BufRead;
            let stdin = std::io::stdin();
            let mut lock = stdin.lock();
            let mut n = 0usize;
            while n < out.len() {
                let mut byte = [0u8; 1];
                match lock.read(&mut byte) {
                    Ok(0) => break,
                    Ok(_) => {
                        out[n] = byte[0];
                        n += 1;
                        if byte[0] == b'\n' {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            n as SQInteger
        } else {
            0
        }
    }
    fn write(&mut self, data: &[u8]) -> SQInteger {
        if let Some(h) = &self.handle {
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
        }
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
        if let Some(h) = &self.handle {
            h.borrow_mut().stream_position().map(|p| p as SQInteger).unwrap_or(-1)
        } else {
            0
        }
    }
    fn len(&mut self) -> SQInteger {
        if let Some(h) = &self.handle {
            let mut f = h.borrow_mut();
            let prev = f.stream_position().unwrap_or(0) as SQInteger;
            let _ = f.seek(SeekFrom::End(0));
            let size = f.stream_position().unwrap_or(0) as SQInteger;
            let _ = f.seek(SeekFrom::Start(prev.max(0) as u64));
            size
        } else {
            0
        }
    }
    fn seek(&mut self, offset: SQInteger, origin: SQInteger) -> SQInteger {
        if let Some(h) = &self.handle {
            let pos = match origin {
                SQ_SEEK_SET => SeekFrom::Start(offset.max(0) as u64),
                SQ_SEEK_CUR => SeekFrom::Current(offset),
                SQ_SEEK_END => SeekFrom::End(offset),
                _ => return -1,
            };
            match h.borrow_mut().seek(pos) {
                Ok(_) => 0,
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
    // 实例字段 __sid（后端注册表索引）
    cls.borrow_mut().new_slot(ss, &ss.intern(b"__sid"), &Value::Integer(0), false);
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
    // 实例字段 __sid（后端注册表索引）
    cls.borrow_mut().new_slot(ss, &ss.intern(b"__sid"), &Value::Integer(0), false);
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
    let len = self_.borrow_mut().len();
    if size > len {
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
    let sid = register_backend(backend);
    set_instance_sid(v, sid);
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
fn blob_swap2(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) => b,
        None => return stream_err(v, "invalid type tag"),
    };
    let mut be = self_.borrow_mut();
    let len = be.len();
    let num = ((len - len % 2) >> 1).max(0) as usize;
    let mut buf = be.snapshot();
    for i in 0..num {
        buf.swap(i * 2, i * 2 + 1);
    }
    be.seek(0, SQ_SEEK_SET);
    be.write(&buf);
    0
}

/// ↔ _blob_swap4：按 32 位双字翻转字节序
fn blob_swap4(v: &mut VmData) -> SQInteger {
    let self_ = match backend_of(v, 1) {
        Some(b) => b,
        None => return stream_err(v, "invalid type tag"),
    };
    let mut be = self_.borrow_mut();
    let len = be.len();
    let num = ((len - len % 4) >> 2).max(0) as usize;
    let mut buf = be.snapshot();
    for i in 0..num {
        let b = i * 4;
        buf.swap(b, b + 3);
        buf.swap(b + 1, b + 2);
    }
    be.seek(0, SQ_SEEK_SET);
    be.write(&buf);
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
    let sid = register_backend(backend);
    set_instance_sid(v, sid);
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
    let sid = register_backend(backend);
    set_instance_sid(v, sid);
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
}

/// ↔ sqstd_createblob：构造 blob 实例并压栈
pub fn push_blob_instance(v: &mut VmData, data: Vec<u8>) -> SQInteger {
    let backend: BackendRef = Rc::new(RefCell::new(BlobBackend::new(0)));
    {
        let mut b = backend.borrow_mut();
        b.write(&data);
        b.seek(0, SQ_SEEK_SET);
    }
    let sid = register_backend(backend);
    let blob_cls = BLOB_CLASS.with(|c| c.borrow().clone());
    if let Some(cls) = blob_cls {
        let inst = SQClass::create_instance(&cls, &v.ss);
        inst.borrow_mut().set(&v.ss.intern(b"__sid"), &Value::Integer(sid));
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
    let sid = register_backend(backend);
    let inst = SQClass::create_instance(file_cls, &v.ss);
    inst.borrow_mut().set(&v.ss.intern(b"__sid"), &Value::Integer(sid));
    v.push(Value::Instance(inst));
}

// ---------------------------------------------------------------------------
// 注册入口
// ---------------------------------------------------------------------------

/// ↔ sqstd_register_iolib 的类声明部分（declare_stream）
/// 返回 (stream_cls, file_cls, blob_cls)
pub fn declare_stream_classes(vm: &mut VmData) -> (ClassRef, ClassRef, ClassRef) {
    let ss = vm.ss.clone();
    let stream_cls = make_stream_class(&ss);
    let file_cls = make_file_class(&ss, &stream_cls);
    let blob_cls = make_blob_class(&ss, &stream_cls);
    BLOB_CLASS.with(|c| *c.borrow_mut() = Some(blob_cls.clone()));

    let root = match &vm.roottable {
        Value::Table(t) => t.clone(),
        _ => return (stream_cls, file_cls, blob_cls),
    };
    // root: stream / file / blob
    root.borrow_mut().new_slot(&ss.intern(b"stream"), &Value::Class(stream_cls.clone()));
    root.borrow_mut().new_slot(&ss.intern(b"file"), &Value::Class(file_cls.clone()));
    root.borrow_mut().new_slot(&ss.intern(b"blob"), &Value::Class(blob_cls.clone()));
    (stream_cls, file_cls, blob_cls)
}
