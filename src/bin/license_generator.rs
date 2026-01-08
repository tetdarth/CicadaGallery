// License key generator tool
// This tool is used by developers to generate signed license keys for users
//
// 使い方:
//   cargo run --bin license_generator                    # 対話モード
//   cargo run --bin license_generator -- --batch 10      # 10件一括生成
//   cargo run --bin license_generator -- --single "User" # 1件生成
//   cargo run --bin license_generator -- --file out.txt --batch 10  # ファイル出力

use ed25519_dalek::{SigningKey, Signer};
use serde::{Deserialize, Serialize};
use base64::Engine;
use chrono::Utc;
use std::io::{self, Write};
use std::env;
use std::fs::File;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub license_type: String,  // "premium", "trial", etc.
    pub issued_to: String,      // Name or email
    pub issued_at: i64,         // Unix timestamp
    pub expires_at: Option<i64>, // Unix timestamp, None for lifetime
}

// ⚠️ 重要: この秘密鍵は安全に保管してください！
// アプリケーション内の公開鍵とペアになっています
const PRIVATE_KEY_HEX: &str = "938d6cbc838342e15ccf9693087acd8e2be6909a01cfdfd580bab1c6c011519b";

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // コマンドライン引数を解析
    let mut batch_count: Option<usize> = None;
    let mut output_file: Option<String> = None;
    let mut single_name: Option<String> = None;
    let mut prefix = "Booth購入者".to_string();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--batch" | "-b" => {
                if i + 1 < args.len() {
                    batch_count = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "--file" | "-f" => {
                if i + 1 < args.len() {
                    output_file = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--single" | "-s" => {
                if i + 1 < args.len() {
                    single_name = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--prefix" | "-p" => {
                if i + 1 < args.len() {
                    prefix = args[i + 1].clone();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {}
        }
        i += 1;
    }
    
    let signing_key = load_private_key(PRIVATE_KEY_HEX);
    
    // バッチモード
    if let Some(count) = batch_count {
        generate_batch_mode(&signing_key, count, &prefix, output_file);
        return;
    }
    
    // シングルモード
    if let Some(name) = single_name {
        let license_key = generate_license_key(&signing_key, &name, None);
        println!("{}", license_key);
        return;
    }
    
    // 対話モード
    run_interactive_mode(&signing_key);
}

fn print_help() {
    println!("CicadaGallery License Key Generator");
    println!();
    println!("使い方:");
    println!("  license_generator                        対話モード");
    println!("  license_generator --batch 10             10件のライセンスを一括生成");
    println!("  license_generator --single \"Name\"        1件のライセンスを生成");
    println!("  license_generator --batch 10 --file out.txt  ファイルに出力");
    println!();
    println!("オプション:");
    println!("  -b, --batch <数>    一括生成するライセンス数");
    println!("  -s, --single <名前> 1件だけ生成（名前を指定）");
    println!("  -f, --file <ファイル> 出力ファイル（バッチモード用）");
    println!("  -p, --prefix <プレフィックス> ライセンス名の接頭辞");
    println!("  -h, --help          このヘルプを表示");
}

fn generate_batch_mode(signing_key: &SigningKey, count: usize, prefix: &str, output_file: Option<String>) {
    let count = count.min(100).max(1);
    
    let mut output: Box<dyn Write> = if let Some(ref path) = output_file {
        match File::create(path) {
            Ok(f) => Box::new(f),
            Err(e) => {
                eprintln!("ファイルの作成に失敗: {}", e);
                return;
            }
        }
    } else {
        Box::new(io::stdout())
    };
    
    // ライセンスキーのみを1行ずつ出力
    for i in 1..=count {
        let issued_to = format!("{} #{:04}", prefix, i);
        let license_key = generate_license_key(signing_key, &issued_to, None);
        writeln!(output, "{}", license_key).unwrap();
    }
    
    if let Some(path) = output_file {
        eprintln!("✅ {} 件のライセンスキーを {} に保存しました", count, path);
    }
}

fn run_interactive_mode(signing_key: &SigningKey) {
    println!("=== CicadaGallery License Key Generator ===\n");
    
    println!("🔐 秘密鍵を読み込みました");
    println!("公開鍵: {}", hex::encode(signing_key.verifying_key().to_bytes()));
    println!("{}\n", "=".repeat(60));
    
    loop {
        println!("\n📝 メニュー:");
        println!("1. 永久ライセンスを生成 (Booth販売用)");
        println!("2. 30日トライアルライセンスを生成");
        println!("3. カスタム期間のライセンスを生成");
        println!("4. 複数のライセンスを一括生成 (Booth用)");
        println!("5. 終了");
        print!("\n選択 (1-5): ");
        io::stdout().flush().unwrap();
        
        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            break;
        }
        
        match choice.trim() {
            "1" => generate_interactive_license(signing_key, None),
            "2" => {
                let expires_at = Utc::now().timestamp() + (30 * 24 * 60 * 60);
                generate_interactive_license(signing_key, Some(expires_at))
            },
            "3" => {
                print!("有効期間（日数）: ");
                io::stdout().flush().unwrap();
                let mut days = String::new();
                io::stdin().read_line(&mut days).unwrap();
                if let Ok(days) = days.trim().parse::<i64>() {
                    let expires_at = Utc::now().timestamp() + (days * 24 * 60 * 60);
                    generate_interactive_license(signing_key, Some(expires_at))
                } else {
                    println!("❌ 無効な数値です");
                }
            },
            "4" => generate_bulk_licenses(signing_key),
            "5" | "" => {
                println!("\n👋 終了します");
                break;
            },
            _ => println!("❌ 無効な選択です"),
        }
    }
}

fn load_private_key(hex_key: &str) -> SigningKey {
    let key_bytes = hex::decode(hex_key).expect("秘密鍵のデコードに失敗");
    let key_array: [u8; 32] = key_bytes.try_into().expect("鍵の長さが無効");
    SigningKey::from_bytes(&key_array)
}

fn generate_interactive_license(signing_key: &SigningKey, expires_at: Option<i64>) {
    print!("\n購入者名またはメールアドレス: ");
    io::stdout().flush().unwrap();
    
    let mut issued_to = String::new();
    io::stdin().read_line(&mut issued_to).unwrap();
    let issued_to = issued_to.trim().to_string();
    
    if issued_to.is_empty() {
        println!("❌ 名前/メールアドレスは必須です");
        return;
    }
    
    let license_key = generate_license_key(signing_key, &issued_to, expires_at);
    print_license(&issued_to, expires_at, &license_key);
}

fn generate_bulk_licenses(signing_key: &SigningKey) {
    print!("\n生成するライセンス数: ");
    io::stdout().flush().unwrap();
    
    let mut count_str = String::new();
    io::stdin().read_line(&mut count_str).unwrap();
    let count: usize = match count_str.trim().parse() {
        Ok(n) if n > 0 && n <= 100 => n,
        _ => {
            println!("❌ 1〜100の数値を入力してください");
            return;
        }
    };
    
    print!("ライセンス名のプレフィックス (例: Booth購入者): ");
    io::stdout().flush().unwrap();
    
    let mut prefix = String::new();
    io::stdin().read_line(&mut prefix).unwrap();
    let prefix = prefix.trim();
    let prefix = if prefix.is_empty() { "Booth Customer" } else { prefix };
    
    println!("\n{}", "=".repeat(60));
    println!("📦 {} 件のライセンスを生成中...", count);
    println!("{}\n", "=".repeat(60));
    
    for i in 1..=count {
        let issued_to = format!("{} #{:04}", prefix, i);
        let license_key = generate_license_key(signing_key, &issued_to, None);
        
        println!("━━━ ライセンス {} ━━━", i);
        println!("発行先: {}", issued_to);
        println!("有効期限: 永久");
        println!("\n{}\n", license_key);
    }
    
    println!("{}", "=".repeat(60));
    println!("✅ {} 件のライセンスを生成しました", count);
    println!("💡 上記のライセンスキーをコピーしてBoothで配布してください");
}

fn generate_license_key(signing_key: &SigningKey, issued_to: &str, expires_at: Option<i64>) -> String {
    let license_info = LicenseInfo {
        license_type: "premium".to_string(),
        issued_to: issued_to.to_string(),
        issued_at: Utc::now().timestamp(),
        expires_at,
    };
    
    // Serialize license info to JSON
    let json_data = serde_json::to_string(&license_info).unwrap();
    let data_bytes = json_data.as_bytes();
    
    // Sign the data
    let signature = signing_key.sign(data_bytes);
    
    // Encode to base64
    let data_b64 = base64::engine::general_purpose::STANDARD.encode(data_bytes);
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    
    // Create license key: base64(data).base64(signature)
    format!("{}.{}", data_b64, signature_b64)
}

fn print_license(issued_to: &str, expires_at: Option<i64>, license_key: &str) {
    println!("\n{}", "=".repeat(60));
    println!("✅ ライセンスキー生成完了!");
    println!("{}", "=".repeat(60));
    println!("📋 タイプ: premium");
    println!("👤 発行先: {}", issued_to);
    println!("📅 発行日: {}", format_timestamp(Utc::now().timestamp()));
    println!("⏰ 有効期限: {}",
        expires_at
            .map(|ts| format_timestamp(ts))
            .unwrap_or_else(|| "永久 (Lifetime)".to_string())
    );
    println!("\n🔑 ライセンスキー:");
    println!("{}", "=".repeat(60));
    println!("{}", license_key);
    println!("{}", "=".repeat(60));
    println!("\n💡 このキーを購入者に送信してください");
    println!("   有効化方法: オプション → ライセンスキーを入力");
}

fn format_timestamp(timestamp: i64) -> String {
    use chrono::DateTime;
    if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        "Invalid date".to_string()
    }
}
