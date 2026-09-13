// 与 callcost.rs 逐行对应的 C++ 版本（MSVC /O2），用于同一台机器上的公平对比。
// 构建：tools\perf\build_callcost.bat
#include <cstdio>
#include <chrono>
#include <vector>
#include <memory>

static const size_t N = 200000000;
volatile unsigned long long INC = 1;

struct Plain { unsigned long long tag; unsigned long long payload; };

struct Vm {
    std::vector<Plain> stack;
    size_t base = 0;
    std::vector<std::shared_ptr<unsigned long long>> rcstack;

    Vm() : stack(64, Plain{0,0}), rcstack(64, std::make_shared<unsigned long long>(0)) {}

    __declspec(noinline) Plain stk_noinline(size_t a) { return stack[base + a]; }
    __declspec(noinline) void set_noinline(size_t a, Plain v) {
        size_t i = base + a;
        if (i >= stack.size()) stack.resize(i + 1, Plain{0,0});
        stack[i] = v;
    }
    __forceinline Plain stk_inline(size_t a) { return stack[base + a]; }
    __forceinline void set_inline(size_t a, Plain v) {
        size_t i = base + a;
        if (i >= stack.size()) stack.resize(i + 1, Plain{0,0});
        stack[i] = v;
    }

    unsigned long long run_noinline() {
        unsigned long long acc = 0;
        for (size_t k = 0; k < N; ++k) {
            Plain a = stk_noinline(0);
            Plain r{ a.tag, a.payload + INC };
            set_noinline(0, r);
            acc = r.payload;
        }
        return acc;
    }
    unsigned long long run_inline() {
        unsigned long long acc = 0;
        for (size_t k = 0; k < N; ++k) {
            Plain a = stk_inline(0);
            Plain r{ a.tag, a.payload + INC };
            set_inline(0, r);
            acc = r.payload;
        }
        return acc;
    }
    unsigned long long run_raw() {
        unsigned long long acc = 0;
        Plain* p = stack.data();
        size_t base_ = base;
        for (size_t k = 0; k < N; ++k) {
            Plain a = p[base_];
            Plain r{ a.tag, a.payload + INC };
            p[base_] = r;
            acc = r.payload;
        }
        return acc;
    }

    __declspec(noinline) std::shared_ptr<unsigned long long> rcget_noinline(size_t a) {
        return rcstack[base + a];   // 计数 +1
    }
    __declspec(noinline) void rcset_noinline(size_t a, std::shared_ptr<unsigned long long> v) {
        rcstack[base + a] = v;      // 旧值计数 -1
    }
    unsigned long long run_rc_noinline() {
        unsigned long long acc = 0;
        for (size_t k = 0; k < N; ++k) {
            auto a = rcget_noinline(0);   // 计数 +1
            auto b = a;                   // 计数 +1
            rcset_noinline(0, b);         // 旧值计数 -1
            acc = *a + INC;
        }
        return acc;
    }
};

static double ns_per_op(double secs) { return secs * 1e9 / (double)N; }

int main() {
    Vm vm;
    auto t0 = std::chrono::steady_clock::now();
    auto a1 = vm.run_noinline();
    auto t1 = std::chrono::steady_clock::now();
    auto a2 = vm.run_inline();
    auto t2 = std::chrono::steady_clock::now();
    auto a3 = vm.run_raw();
    auto t3 = std::chrono::steady_clock::now();
    auto a4 = vm.run_rc_noinline();
    auto t4 = std::chrono::steady_clock::now();

    auto d = [](std::chrono::steady_clock::time_point a, std::chrono::steady_clock::time_point b) {
        return std::chrono::duration<double>(b - a).count();
    };
    printf("cpp   noinline  : %7.2f ns/op   (chk %llu)\n", ns_per_op(d(t0,t1)), a1);
    printf("cpp   inline    : %7.2f ns/op   (chk %llu)\n", ns_per_op(d(t1,t2)), a2);
    printf("cpp   raw+local : %7.2f ns/op   (chk %llu)\n", ns_per_op(d(t2,t3)), a3);
    printf("cpp   rc-payload: %7.2f ns/op   (chk %llu)\n", ns_per_op(d(t3,t4)), a4);
    return 0;
}
