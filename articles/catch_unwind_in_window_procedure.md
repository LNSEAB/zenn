---
title: "Windowsのウィンドウプロシージャ内におけるRustのパニック"
emoji: "🏉"
type: "tech" # tech: 技術記事 / idea: アイデア
topics: ["Rust", "Windows", "FFI"]
published: true 
---

RustのパニックがFFI境界を跨いで巻き戻そうとすると未定義動作となります[^1]。Windowsでウィンドウを作る場合のウィンドウプロシージャもこの例に漏れずウィンドウプロシージャとメッセージループの間がFFI境界となるので、対策なしにウィンドウプロシージャでパニックが起きると未定義動作となります。
ここで`std::panic::catch_unwind`[^2](以下`catch_unwind`）と`std::panic::resume_unwind`[^3]（以下`resume_unwind`）を用いることで未定義動作を起こさないようにします。

## ウィンドウプロシージャとcatch_unwind

ウィンドウプロシージャのメッセージ処理を`catch_unwind`のクロージャ内で行うことで、その中で起きたパニックが`catch_unwind`の返り値に格納されてパニックがFFI境界を超えることを防ぎます。

```rust
use std::panic::catch_unwind;
use std::cell::RefCell;
use std::any::Any;

thread_local! {
    static UNWIND: RefCell<Option<Box<dyn Any + Send>>> = RefCell::new(None);
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    catch_unwind(|| {
        // WM_DESTROY等のメッセージの処理はcatch_unwindの中で
    })
    .unwrap_or_else(|e| {
        // catch_unwindの中で起きたパニックをUNWINDに入れる
        UNWIND.with(|unwind| *unwind.borrow_mut() = Some(e));
        // とりあえずメッセージの処理したことにしてウィンドウプロシージャから抜ける
        0
    })
}
```

## メッセージループとresume_unwind

ウィンドウプロシージャで起きたパニックをメッセージループの中で`resume_unwind`を用いて再開させます。

```rust
use std::panic::resume_unwind;
use std::ptr::null_mut;
use winapi::um::winuser::*;

unsafe {
    let mut msg = MSG::default();
    loop {
        let ret = GetMessageW(&mut msg, null_mut(), 0, 0);
        if ret == 0 || ret == -1 {
            break;
        }
        DispatchMessageW(&msg);
        // ウィンドウプロシージャで起きたパニックはここで再開される
        UNWIND.with(|unwind| {
            if let Some(e) = unwind.borrow_mut().take() {
                resume_unwind(e);
            }
        });
    }
}
```

## WM_CREATE

`WM_CREATE`でパニックが起きた場合を考え、`CreateWindowEx`の次に`resume_unwind`するかどうかをチェックします。`WM_CREATE`の時に-1を返すと`CreateWindowEx`はNULLを返すことが決まっているため、NULLチェックの前にパニックのチェックをする必要があります。

```rust
use winapi::um::winuser::*;

unsafe {
    let hwnd = CreateWindowExW(
        // 省略
    );
    UNWIND.with(|unwind| {
        if let Some(e) = unwind.borrow_mut().take() {
            resume_unwind(e);
        }
    });
    if hwnd == null_mut() {
        panic!("CreateWindowEx failed");
    }
}
```

## サンプル

マウスの右ボタンを押して離した瞬間にパニックを起こします。
また、`cargo run --features=panic_create`とすると`WM_CREATE`でパニックを起こすようになります。

https://github.com/LNSEAB/zenn/tree/main/samples/catch_unwind_in_window_procedure

[^1]: FFI and panics - The Rustonomicon 
https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-panics

[^2]: std::panic::catch_unwind
https://doc.rust-lang.org/std/panic/fn.catch_unwind.html

[^3]: std::panic::resume_unwind
https://doc.rust-lang.org/std/panic/fn.resume_unwind.html