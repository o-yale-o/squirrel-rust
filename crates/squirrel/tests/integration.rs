//! 端到端集成测试：编译 → 执行 → 捕获 print 输出。

use squirrel::api::Vm;
use std::cell::RefCell;
use std::rc::Rc;

fn run_script(src: &str) -> String {
    let out = Rc::new(RefCell::new(String::new()));
    let vm = Vm::new(1024);
    {
        let o2 = out.clone();
        let o3 = out.clone();
        vm.set_print_funcs(
            Some(Rc::new(move |_v, s: &[u8]| {
                o2.borrow_mut().push_str(&String::from_utf8_lossy(s))
            })),
            Some(Rc::new(move |_v, s: &[u8]| {
                o3.borrow_mut().push_str(&String::from_utf8_lossy(s));
            })),
        );
    }
    match vm.run_buffer(src.as_bytes(), b"test.nut") {
        Ok(_) => {}
        Err(e) => out.borrow_mut().push_str(&format!("<error: {}>", e)),
    }
    let r = out.borrow().clone();
    r
}

#[test]
fn hello_world() {
    assert_eq!(run_script(r#"print("Hello World!\n")"#), "Hello World!\n");
}

#[test]
fn arithmetic() {
    assert_eq!(
        run_script(r#"print(1+2*3); print("\n"); print(10/4); print("\n"); print(7%3); print("\n")"#),
        "7\n2\n1\n"
    );
}

#[test]
fn floats_format_like_crt() {
    assert_eq!(
        run_script(r#"print(1.0/3.0); print("\n"); print(1000000.0); print("\n"); print(0.1)"#),
        "0.333333\n1e+06\n0.1"
    );
}

#[test]
fn locals_and_control_flow() {
    assert_eq!(
        run_script(
            r#"local s = 0;
for(local i=1; i<=10; i++) s += i;
print(s); print("\n");
local x = 5; if (x > 3) print("big"); else print("small");
print("\n");
local i = 0; while(i < 3) { print(i); i++; }"#
        ),
        "55\nbig\n012"
    );
}

#[test]
fn functions_and_recursion() {
    assert_eq!(
        run_script(
            r#"function fact(n) { if (n <= 1) return 1; return n * fact(n-1); }
print(fact(10)); print("\n");
function fib(n) { return n < 2 ? n : fib(n-1) + fib(n-2); }
print(fib(20)); print("\n");"#
        ),
        "3628800\n6765\n"
    );
}

#[test]
fn tables_arrays_foreach() {
    assert_eq!(
        run_script(
            r#"local t = {a = 1, b = 2};
t.c <- 3;
foreach(k, v in t) print(k + "=" + v + " ");
print("\n");
local a = [10, 20, 30];
foreach(i, v in a) print(i + ":" + v + " ");
print("\n");
print(a.len()); print("\n");
print(t.len()); print("\n");"#
        ),
        "a=1 c=3 b=2 \n0:10 1:20 2:30 \n3\n3\n"
    );
}

#[test]
fn closures_capture() {
    assert_eq!(
        run_script(
            r#"local function make_counter() {
    local n = 0;
    return function() { n++; return n; };
}
local c = make_counter();
print(c()); print(c()); print(c());"#
        ),
        "123"
    );
}

#[test]
fn classes_and_inheritance() {
    assert_eq!(
        run_script(
            r#"class Animal {
    name = null;
    constructor(n) { name = n; }
    function speak() { return "silence"; }
}
class Dog extends Animal {
    function speak() { return name + " says woof"; }
}
local d = Dog("rex");
print(d.speak()); print("\n");
print(d instanceof Animal); print("\n");
print(d instanceof Dog);"#
        ),
        "rex says woof\ntrue\ntrue"
    );
}

#[test]
fn try_catch_throw() {
    assert_eq!(
        run_script(
            r#"try {
    throw "boom";
} catch(e) {
    print("caught: " + e);
}
print("\n");
try {
    local t = {};
    local x = t.missing;
} catch(e) {
    print("caught2");
}"#
        ),
        "caught: boom\ncaught2"
    );
}

#[test]
fn string_methods() {
    assert_eq!(
        run_script(
            r#"print("hello".toupper()); print("\n");
print("ABC".tolower()); print("\n");
print("hello world".slice(6)); print("\n");
print("hello".find("ll")); print("\n");
print("abc".len()); print("\n");
print("42".tointeger() + 1); print("\n");
print("3.5".tofloat() * 2);"#
        ),
        "HELLO\nabc\nworld\n2\n3\n43\n7"
    );
}

#[test]
fn array_methods() {
    assert_eq!(
        run_script(
            r#"local a = [3, 1, 2];
a.sort();
foreach(v in a) print(v);
print("\n");
a.reverse();
foreach(v in a) print(v);
print("\n");
print(a.top()); print("\n");
local m = [1,2,3].map(function(v) { return v * 10 });
foreach(v in m) print(v + ",");"#
        ),
        "123\n321\n1\n10,20,30,"
    );
}

#[test]
fn generators() {
    assert_eq!(
        run_script(
            r#"function range(n) {
    for(local i = 0; i < n; i++) {
        yield i;
    }
    return null;
}
foreach(v in range(5)) print(v);
print("\n");
local g = range(3);
print(resume g);
print(resume g);
print(g.getstatus());"#
        ),
        "01234\n01suspended"
    );
}

#[test]
fn varargs_and_default_params() {
    assert_eq!(
        run_script(
            r#"function va(a, ...) {
    local s = a;
    foreach(v in vargv) s += v;
    return s;
}
print(va(1, 2, 3, 4)); print("\n");
function dp(a, b = 10, c = 20) { return a + b + c; }
print(dp(1)); print(" "); print(dp(1, 2)); print(" "); print(dp(1, 2, 3));"#
        ),
        "10
31 23 6"
    );
}

#[test]
fn weakref_and_typeof() {
    assert_eq!(
        run_script(
            r#"local t = {};
local w = t.weakref();
print(typeof w); print("\n");
print(typeof 1); print(typeof 1.5); print(typeof "s"); print(typeof t); print(typeof print);"#
        ),
        "weakref\nintegerfloatstringtablefunction"
    );
}

#[test]
fn error_reporting() {
    let out = run_script(r#"local x = null; x.y;"#);
    assert!(out.contains("<error:") || out.contains("does not exist"), "got: {}", out);
}

#[test]
fn comments_and_line_numbers() {
    assert_eq!(
        run_script(
            r#"// line 1
# line 2
/* block
   comment */
local x = __LINE__; // 5
print(x);"#
        ),
        "5"
    );
}
