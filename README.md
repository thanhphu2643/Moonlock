# 🌙 Moonlock

> Khoá deploy theo mặt trăng.

Lịch âm cho lập trình viên: **nhắc rằm, mùng 1, giỗ chạp** và **tự chặn deploy vào ngày "kiêng"**.
Rust + Swift, mã nguồn mở (MIT), chạy đa nền tảng với **Tauri 2**: macOS · Windows · Linux · **iOS** · **Android**.

```
$ moonlock deploy-check
📅 Chủ nhật, 01/11/2026  ·  🌙 23/9 năm Bính Ngọ
   Ngày Kỷ Mão · tháng Mậu Tuất · hắc đạo (Câu Trận)
🚦 ⛔ CẤM DEPLOY
   ⛔ Nguyệt Kỵ — "mùng 5, 14, 23, đi chơi còn lỗ nữa là deploy"
👉 Ngày deploy gần nhất: Thứ 2, 2026-11-02 (24/9 âm)
$ echo $?
1
```

## Tính năng

- **Âm lịch Việt Nam chuẩn** (thuật toán Hồ Ngọc Đức, UTC+7): tháng nhuận, can chi ngày/tháng/năm, trực tinh hoàng đạo / hắc đạo.
- **Luật kiêng** (mỗi luật chọn được *Chặn* / *Cảnh báo* / *Bỏ qua*):
  `mung_mot`, `ram`, `tam_nuong` (3·7·13·18·22·27), `nguyet_ky` (5·14·23), `hac_dao`, `gio`, `tet` (23 Chạp → mùng 7 Giêng), `le_lon`, `friday`, `weekend`.
- **Giỗ chạp** theo ngày âm, tự lặp mỗi năm; giỗ ngày 30 mà tháng thiếu thì nhắc ngày 29; tháng nhuận thì cúng theo tháng thường.
- **Desktop app (Tauri 2)**: lịch tháng có tô màu, thẻ "Hôm nay deploy được không?", quản lý giỗ, chỉnh luật, thông báo hệ thống trước ngày rằm/giỗ, tray hiển thị ngày âm (`🌙 23/9 ⛔`).
- **App mobile (iOS & Android)**: cùng giao diện, bố cục riêng cho điện thoại (tab bar dưới, lịch thu gọn, bottom sheet thêm giỗ). Nhắc rằm/mùng 1/giỗ được **lên lịch sẵn bằng local notification** của hệ điều hành nên vẫn báo khi app đã tắt.
- **CLI `moonlock`**, dùng làm chốt chặn trong CI/CD: exit 1 vào ngày kiêng, có ghi summary cho GitHub Actions.
- **Export `.ics`**: ngày cấm deploy (busy) + nhắc lễ/giỗ (có báo trước), import được vào Google/Outlook/Apple Calendar.
- **Apple Calendar (Swift/EventKit)**: trên macOS và iOS ghi thẳng vào lịch riêng "Moonlock", đồng bộ iCloud sang iPhone/Watch, chạy lại nhiều lần không tạo trùng.

## Kiến trúc

```
crates/moonlock-core   Rust: chuyển đổi âm↔dương, can chi, luật kiêng, policy, ICS (không phụ thuộc UI)
crates/moonlock-cli    Rust: CLI `moonlock` (today, month, upcoming, deploy-check, ics, lunar, init)
swift/MoonlockKit      Swift: EventKit → Apple Calendar, export qua C ABI (@_cdecl), dùng chung macOS + iOS
app/src-tauri          Rust: Tauri 2 shell (commands), link Swift qua swift-rs cho target Apple
  src/desktop.rs       tray hiển thị ngày âm + vòng lặp nhắc (desktop)
  src/mobile.rs        lên lịch local notification 45 ngày tới (iOS/Android)
  gen/apple            project Xcode (XcodeGen: project.yml)
  gen/android          project Android Studio (Gradle)
app/ui                 HTML/CSS/JS thuần, không bundler; responsive cho điện thoại
action.yml           GitHub Action composite
```

Cấu hình của app và CLI là **cùng một định dạng TOML**. Copy từ tab *Tích hợp* trong app vào `.moonlock.toml` ở repo, CI sẽ chặn đúng những ngày bạn thấy trong app.

## Chạy

Cần Rust (stable), Node 18+. Trên macOS cần thêm Xcode Command Line Tools (để build Swift).
Linux: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`.

```bash
# App desktop
cd app && npm install
npm run dev          # chạy dev
npm run build        # đóng gói .app/.dmg, .msi/.exe, .deb/.AppImage

# iOS (cần Xcode + iOS Simulator runtime khớp SDK)
npm run tauri ios dev                       # simulator / thiết bị
npm run tauri ios build -- --no-sign        # .ipa chưa ký (ký bằng Apple Developer để cài)

# Android (cần JDK 17, Android SDK + NDK)
export JAVA_HOME=/opt/homebrew/opt/openjdk@17 NDK_HOME=$ANDROID_HOME/ndk/<phiên-bản>
npm run tauri android dev                   # emulator / thiết bị
npm run tauri android build -- --apk        # .apk / .aab

# CLI
cargo install --git https://github.com/thanhphu2643/Moonlock moonlock-cli   # hoặc: cargo install --path crates/moonlock-cli
moonlock               # hôm nay
moonlock month 2027-02
moonlock upcoming --days 90
moonlock lunar 15/8/2027
moonlock init          # tạo .moonlock.toml
moonlock ics -o lich.ics

# Test
cargo test -p moonlock-core
```

> **Lưu ý toolchain cho mobile:** cần Rust cài qua **rustup** (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`). Rust của Homebrew không có thư viện chuẩn cho iOS/Android — nếu máy có cả hai, đặt `PATH="$(brew --prefix rustup)/bin:$PATH"` khi build mobile.

## Chặn deploy trong CI

**GitHub Actions**

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: thanhphu2643/Moonlock@main       # exit 1 nếu hôm nay kiêng
        with:
          strict: false                    # true = chặn cả ngày "cảnh báo"
      - run: ./deploy.sh
```

**GitLab CI / bất kỳ đâu**

```bash
moonlock deploy-check || exit 1
```

**Khẩn cấp** (prod cháy, kệ ngày kiêng):

```bash
MOONLOCK_ALLOW_DEPLOY="đã thắp hương xin phép" moonlock deploy-check
```

## Cấu hình `.moonlock.toml`

```toml
timezone = 7.0
remind_days_before = 1

[deploy]
block = ["mung_mot", "ram", "tam_nuong", "nguyet_ky", "gio", "tet"]
warn  = ["hac_dao", "le_lon", "friday"]

[[gio]]
name = "Ông nội"
day = 12
month = 3
```

Xem `.moonlock.toml.example`. App lưu cấu hình ở thư mục config của hệ điều hành
(macOS: `~/Library/Application Support/io.github.thanhphu2643.moonlock/moonlock.toml`; iOS/Android: thư mục dữ liệu riêng của app).
Trên iOS, file `.ics` export nằm trong app **Files → Trên iPhone → Moonlock**.

## Lộ trình

- [x] iOS/Android qua Tauri 2 mobile
- [ ] Đồng bộ Google Calendar / lịch hệ thống trên Android (CalendarContract, Kotlin)
- [ ] Phát hành TestFlight / Google Play (cần tài khoản developer + ký số)
- [ ] Widget macOS (WidgetKit, Swift) hiển thị ngày âm + trạng thái deploy
- [ ] Giờ hoàng đạo (chặn deploy theo giờ)
- [ ] Webhook Slack/Discord: "Hôm nay rằm, ai deploy là tự chịu"

## Ghi chú

Đây là công cụ vui cho dev, đề cao văn hoá. Các luật kiêng theo dân gian, không phải lời khuyên tâm linh.
Thuật toán âm lịch dựa trên công trình của Hồ Ngọc Đức.
