# CicadaGallery ビルド手順書

## 概要

CicadaGalleryはCargoのfeatureフラグを使用して、無料版とプレミアム版を分離しています。

| バージョン | feature | ライセンス検証 | プレミアム機能 |
|------------|---------|----------------|----------------|
| 無料版 | なし（デフォルト） | 無効 | 無効 |
| プレミアム版 | `premium` | 有効 | 有効 |

---

## 前提条件

- Rust 1.70以上
- Windows 10/11 (64-bit)

---

## 無料版のビルド（公開用）

公開リポジトリからビルドする場合：

```bash
# 開発ビルド
cargo build

# リリースビルド
cargo build --release
```

実行ファイル: `target/release/cicada_gallery.exe`

### 特徴
- ライセンス検証コードが含まれない
- `is_premium` は常に `false`
- プレミアム機能のUIは表示されるが、使用不可

---

## プレミアム版のビルド（開発者専用）

### 必要なファイル

プレミアム版をビルドするには、以下のファイルが必要です：

```
src/
└── license_premium.rs  # ライセンス検証ロジック（秘密鍵含む）
```

> **Note**: `license_premium.rs` は `.gitignore` で除外されているため、
> 公開リポジトリには含まれません。

### ビルドコマンド

```bash
# 開発ビルド
cargo build --features premium

# リリースビルド
cargo build --release --features premium
```

実行ファイル: `target/release/cicada_gallery.exe`

---

## ファイル構成

```
src/
├── license.rs          # 公開用ライセンスモジュール
│                       # premium featureなし: 常にエラーを返す
│                       # premium featureあり: license_premium.rsを呼び出す
│
├── license_premium.rs  # プレミアム用（非公開）
│                       # 実際のライセンス検証ロジック
│                       # 公開鍵による署名検証
│
└── bin/
    └── license_generator.rs  # ライセンスキー生成ツール（非公開）
```

---

## ライセンスキー生成（開発者専用）

ライセンスキーを生成するには：

```bash
cargo run --bin license_generator --features premium
```

---

## 公開リポジトリへのプッシュ

### 初回設定

以下のファイルをGit追跡から除外：

```bash
# 追跡を解除（ファイルは残す）
git rm --cached src/license_premium.rs
git rm --cached src/bin/license_generator.rs
git rm --cached licenses.txt

# コミット
git commit -m "Remove premium files from tracking"
```

### .gitignore 設定

```gitignore
# Premium module (not included in public repository)
src/license_premium.rs
src/bin/license_generator.rs
licenses.txt
```

---

## セキュリティ

### なぜ安全か？

1. **ライセンス検証コードが存在しない**
   - 無料版には `license_premium.rs` が含まれない
   - 検証ロジックがないため、改ざんしてもプレミアム機能を有効化できない

2. **署名検証**
   - ライセンスキーは秘密鍵で署名
   - 公開鍵のみがアプリに埋め込まれる
   - 秘密鍵がなければ有効なライセンスキーを生成できない

3. **featureフラグによる分離**
   - `#[cfg(feature = "premium")]` でコードがコンパイル時に除外
   - バイナリに含まれないため逆アセンブルしても発見できない

---

## トラブルシューティング

### ビルドエラー: `license_premium.rs` が見つからない

プレミアム版をビルドするには、プライベートリポジトリから `license_premium.rs` を取得してください。

無料版をビルドする場合は `--features premium` を外してください：

```bash
cargo build --release
```

### ライセンス検証が失敗する

無料版ではライセンス検証は常に失敗します。これは仕様です。

---

## チェックリスト

### 公開前チェック

- [ ] `src/license_premium.rs` が `.gitignore` に含まれている
- [ ] `src/bin/license_generator.rs` が `.gitignore` に含まれている  
- [ ] `licenses.txt` が `.gitignore` に含まれている
- [ ] `cargo build` (featureなし) でビルドできる
- [ ] プレミアム機能が無効になっている

### リリースビルドチェック

- [ ] `cargo build --release --features premium` でビルドできる
- [ ] ライセンス検証が動作する
- [ ] プレミアム機能が有効になる
