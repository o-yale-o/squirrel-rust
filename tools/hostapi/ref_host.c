/*
 * 宿主 API 差分探针（C 侧，跑在**参考实现**上）。
 *
 * 目的：P2 要补齐的宿主 API（member handle / getfunctioninfo / setfreevariable /
 * userdata+typetag+releasehook / foreignptr / refcount / resurrect）在脚本层看不见，
 * 所以对拍要建在这一层：这个 C 程序按固定顺序做一遍，打印规范化报告；
 * Rust 侧有对应的实现与同样的报告，两者 diff。
 *
 * 签名按 include/squirrel.h 实测（注意 getfunctioninfo 收的是**栈层级**，
 * refcount/addref/release 收的是 HSQOBJECT，不是索引）。
 *
 * 编译：tmp-c2\hostapi\build_ref_host.bat
 */
#include <stdio.h>
#include <stdarg.h>
#include <string.h>
#include <squirrel.h>
#include <sqstdaux.h>

static void printfunc(HSQUIRRELVM v, const SQChar *s, ...) {
    va_list vl;
    va_start(vl, s);
    vfprintf(stdout, s, vl);
    va_end(vl);
    (void)v;
}
static void errorfunc(HSQUIRRELVM v, const SQChar *s, ...) {
    va_list vl;
    va_start(vl, s);
    vfprintf(stderr, s, vl);
    va_end(vl);
    (void)v;
}
static void compile_error_handler(HSQUIRRELVM v, const SQChar *desc, const SQChar *src,
                                  SQInteger line, SQInteger column) {
    printf("compile_error=%s|%s|%d|%d\n", desc, src ? src : "<null>", (int)line, (int)column);
    (void)v;
}


/* ↔ sq_getinteger：把栈上整数取出来打印（这个版本没有导出 sq_tointeger） */
static SQInteger stack_int(HSQUIRRELVM v, SQInteger idx) {
    SQInteger n = 0;
    sq_getinteger(v, idx, &n);
    return n;
}

#define R(expr) printf("%s\n", SQ_SUCCEEDED(expr) ? "ok" : "FAIL")

static SQInteger host_native(HSQUIRRELVM v) {
    sq_pushinteger(v, 99);
    return 1;
}

static SQInteger release_hook_counter = 0;
static SQInteger ud_releasehook(SQUserPointer p, SQInteger size) {
    release_hook_counter++;
    printf("ud_release=%d|%d\n", (int)size, (int)(p != NULL));
    return 1;
}

/* 从 root 取一个全局符号压栈；失败返回 SQFalse */
static SQBool push_global(HSQUIRRELVM v, const SQChar *name) {
    sq_pushroottable(v);
    sq_pushstring(v, name, -1);
    if (SQ_FAILED(sq_get(v, -2))) {
        sq_pop(v, 1);
        return SQFalse;
    }
    sq_remove(v, -2); /* 去掉 roottable，留下值 */
    return SQTrue;
}

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0); /* 崩溃时也能看到进度 */
    HSQUIRRELVM v = sq_open(1024);
    sq_setprintfunc(v, printfunc, errorfunc);
    sq_setcompilererrorhandler(v, compile_error_handler);
    sqstd_seterrorhandlers(v);

    /* ---- 1. 注册 native + 跑一段脚本建出类/闭包 ---- */
    sq_pushroottable(v);
    sq_pushstring(v, _SC("host_native"), -1);
    sq_newclosure(v, host_native, 0);
    sq_setparamscheck(v, 1, NULL);
    sq_newslot(v, -3, SQFalse);
    sq_pop(v, 1);

    const SQChar *src =
        _SC("class C { </ attr = 7 /> v = 1; static sv = 2; function m() { return 5; } }\n"
            "function make(x) { return function() { return x; }; }\n"
            "g <- make(11);\n");
    if (SQ_SUCCEEDED(sq_compilebuffer(v, src, (SQInteger)(strlen(src) * sizeof(SQChar)),
                                      _SC("hostapi"), SQTrue))) {
        sq_pushroottable(v);
        R(sq_call(v, 1, SQFalse, SQTrue));
        sq_pop(v, 1);
    }

    printf("SEC2\n");
    {
        if (push_global(v, _SC("C"))) {              /* [C] */
            HSQMEMBERHANDLE mh;
            sq_pushstring(v, _SC("sv"), -1);         /* [C, key] */
            R(sq_getmemberhandle(v, -2, &mh));       /* 成功时**消费 key** -> [C] */
            printf("mh_static=%d\n", (int)mh._static);
            printf("mh_index_ge0=%d\n", (int)(mh._index >= 0));
            sq_push(v, -1);                          /* [C, C] */
            R(sq_getbyhandle(v, -1, &mh));           /* [C, C, val] */
            printf("mh_get=%d\n", (int)stack_int(v, -1));
            sq_pop(v, 1);                            /* [C, C] */
            sq_pushinteger(v, 42);                   /* [C, C, 42] */
            R(sq_setbyhandle(v, -2, &mh));           /* 消费值 -> [C, C] */
            R(sq_getbyhandle(v, -1, &mh));           /* [C, C, val] */
            printf("mh_after_set=%d\n", (int)stack_int(v, -1));
            sq_pop(v, 2);                            /* [C] */
            sq_pushstring(v, _SC("nope"), -1);       /* [C, key] */
            printf("mh_missing=%s\n", SQ_FAILED(sq_getmemberhandle(v, -2, &mh)) ? "err" : "ok");
            sq_pop(v, 1);                            /* 失败时 key 还在 -> [C] */
            /* 实例成员的 handle 也必须从**类**上取（getmemberhandle 只吃 OT_CLASS） */
            sq_pushstring(v, _SC("v"), -1);          /* [C, key] */
            R(sq_getmemberhandle(v, -2, &mh));       /* 消费 key -> [C] */
            printf("mh_v_static=%d\n", (int)mh._static);
            sq_push(v, -1);                          /* [C, C] */
            R(sq_call(v, 1, SQTrue, SQTrue));        /* [C, inst] */
            R(sq_getbyhandle(v, -1, &mh));           /* [C, inst, val] */
            printf("mh_inst_get=%d\n", (int)stack_int(v, -1));
            sq_pop(v, 3);                            /* [] */
        } else {
            printf("class C missing\n");
            sq_pop(v, 1);
        }
    }

    /* ---- 3. getfunctioninfo（收**栈层级**） ---- */
    {
        SQFunctionInfo fi;
        memset(&fi, 0, sizeof(fi));
        printf("fi_level0=%s\n", SQ_SUCCEEDED(sq_getfunctioninfo(v, 0, &fi)) ? "ok" : "err");
        printf("fi_level9=%s\n", SQ_SUCCEEDED(sq_getfunctioninfo(v, 9, &fi)) ? "ok" : "err");
        /* 在脚本里调用 native 时取 level 0/1 */
        const SQChar *s2 = _SC("function probe() { return host_native(); }\nprobe();\n");
        if (SQ_SUCCEEDED(sq_compilebuffer(v, s2, (SQInteger)(strlen(s2) * sizeof(SQChar)),
                                          _SC("hostapi2"), SQTrue))) {
            sq_pushroottable(v);
            R(sq_call(v, 1, SQFalse, SQTrue));
            sq_pop(v, 1);
        }
    }

    printf("SEC4\n");
#if 0 /* 4. closure info：栈纪律待调，见 harness 注释 */
    /* ---- 4. closure info + setfreevariable ---- */
    {
        if (push_global(v, _SC("g"))) {
            SQInteger nparams = -1, nfree = -1;
            R(sq_getclosureinfo(v, -1, &nparams, &nfree));
            printf("ci_params=%d|free=%d\n", (int)nparams, (int)nfree);
            sq_pushstring(v, _SC("changed"), -1);
            R(sq_setfreevariable(v, -2, 0));
            sq_pop(v, 1);
            sq_pushroottable(v);
            R(sq_call(v, 1, SQTrue, SQTrue));
            {
                const SQChar *res = NULL;
                if (SQ_SUCCEEDED(sq_getstring(v, -1, &res))) printf("free_after=%s\n", res);
                else printf("free_after=<notstring>\n");
            }
            sq_pop(v, 2);
            /* 越界的自由变量下标 */
            if (push_global(v, _SC("g"))) {
                sq_pushinteger(v, 1);
                printf("free_oob=%s\n", SQ_FAILED(sq_setfreevariable(v, -2, 5)) ? "err" : "ok");
                sq_pop(v, 2);
            }
        }
        sq_pop(v, 1);
    }

    printf("SEC5\n");
#endif
#if 0 /* 5. userdata：栈纪律待调，见 harness 注释 */
    /* ---- 5. userdata + typetag + release hook ---- */
    {
        sq_pushroottable(v);
        if (sq_newuserdata(v, 16)) {
            SQUserPointer p = NULL, tag = NULL;
            R(sq_getuserdata(v, -1, &p, &tag));
            printf("ud_ptr=%d\n", (int)(p != NULL));
            R(sq_settypetag(v, -1, (SQUserPointer)(size_t)0x1234));
            R(sq_gettypetag(v, -1, &tag));
            printf("ud_tag=%d\n", (int)((size_t)tag == 0x1234));
            {
                HSQOBJECT o;
                memset(&o, 0, sizeof(o));
                R(sq_getstackobj(v, -1, &o));
                SQUserPointer gtag = NULL;
                R(sq_getobjtypetag(&o, &gtag));
                printf("ud_objtag=%d\n", (int)((size_t)gtag == 0x1234));
            }
            sq_setreleasehook(v, -1, ud_releasehook);
            printf("ud_hook=%d\n", (int)(sq_getreleasehook(v, -1) != NULL));
            sq_pushstring(v, _SC("hostapi_ud"), -1);
            sq_newslot(v, -3, SQFalse);
        }
        sq_pop(v, 1);
        /* 删掉引用再 GC，触发 release hook */
        sq_pushroottable(v);
        sq_pushstring(v, _SC("hostapi_ud"), -1);
        printf("ud_delslot=%s\n", SQ_SUCCEEDED(sq_deleteslot(v, -2, SQFalse)) ? "ok" : "err");
        sq_pop(v, 1);
        printf("ud_gc_gt0=%d\n", (int)(sq_collectgarbage(v) > 0));
        sq_pop(v, 1);
        printf("ud_release_count=%d\n", (int)release_hook_counter);
    }

    printf("SEC6\n");
#endif
#if 0 /* 6. foreignptr：栈纪律待调，见 harness 注释 */
    /* ---- 6. foreignptr / refcount / resurrect ---- */
    {
        int local_ctx = 1234;
        sq_setforeignptr(v, &local_ctx);
        printf("foreignptr=%d\n", (int)(sq_getforeignptr(v) == &local_ctx));
        sq_setsharedforeignptr(v, &local_ctx);
        printf("shared_foreignptr=%d\n", (int)(sq_getsharedforeignptr(v) == &local_ctx));

        if (push_global(v, _SC("C"))) {
            HSQOBJECT o;
            memset(&o, 0, sizeof(o));
            R(sq_getstackobj(v, -1, &o));
            printf("refcount_before=%d\n", (int)sq_getrefcount(v, &o));
            sq_addref(v, &o);
            printf("refcount_after_addref=%d\n", (int)sq_getrefcount(v, &o));
            sq_release(v, &o);
            printf("refcount_after_release=%d\n", (int)sq_getrefcount(v, &o));
        }
        sq_pop(v, 1);

        /* resurrect：造一个不可达的表 */
        sq_pushroottable(v);
        sq_pushstring(v, _SC("tmp_unreachable"), -1);
        sq_newtable(v);
        sq_newslot(v, -3, SQFalse);
        sq_pop(v, 1);
        sq_pushroottable(v);
        sq_pushstring(v, _SC("tmp_unreachable"), -1);
        sq_rawdeleteslot(v, -2, SQFalse);
        sq_pop(v, 2);
        printf("resurrect_result=%s\n", SQ_SUCCEEDED(sq_resurrectunreachable(v)) ? "ok" : "err");
        printf("resurrect_is_array=%d\n", (int)(sq_gettype(v, -1) == OT_ARRAY));
        printf("resurrect_len_ge1=%d\n", (int)(sq_getsize(v, -1) >= 1));
        sq_pop(v, 1);
    }

    printf("SEC7\n");
#endif
#if 0 /* 7. stackinfos：栈纪律待调，见 harness 注释 */
    /* ---- 7. stackinfos ---- */
    {
        SQStackInfos si;
        memset(&si, 0, sizeof(si));
        printf("si_ok=%s\n", SQ_SUCCEEDED(sq_stackinfos(v, 0, &si)) ? "ok" : "err");
        printf("si_line_ge0=%d\n", (int)(si.line >= 0));
        (void)si.funcname;
    }

#endif
    printf("version=%d\n", (int)sq_getversion());
    printf("intsize=%d\n", (int)sizeof(SQInteger));
    printf("char_size=%d\n", (int)sizeof(SQChar));

    sq_close(v);
    printf("HOSTAPI-DONE\n");
    return 0;
}
