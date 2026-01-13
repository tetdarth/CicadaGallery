---
layout: default
title: Premium License
lang: ja
---

**🇯🇵 日本語** | [🇺🇸 English](en/license.md)

# 🎫 ライセンス発行

Gumroadでご購入いただいた方は、以下のフォームからライセンスキーを取得できます。

<div id="license-app">
  <div class="form-container">
    <div class="form-group">
      <label for="order_id">注文ID (Order ID)</label>
      <input type="text" id="order_id" placeholder="例: XXXXXXXXXX" required>
      <small>Gumroadからの購入完了メールに記載されています</small>
    </div>
    
    <div class="form-group">
      <label for="email">メールアドレス</label>
      <input type="email" id="email" placeholder="購入時のメールアドレス" required>
    </div>
    
    <button id="submit-btn" onclick="submitForm()">ライセンスを発行</button>
    
    <div id="message"></div>
  </div>
  
  <div class="help-section">
    <details>
      <summary>❓ 注文IDの確認方法</summary>
      <div class="help-content">
        <h4>📧 メールで確認</h4>
        <ol>
          <li>Gumroadから届いた「Receipt for your purchase」メールを開く</li>
          <li>メール内に記載されている注文IDをコピー</li>
        </ol>
        
        <h4>📚 Gumroadライブラリで確認</h4>
        <ol>
          <li><a href="https://app.gumroad.com/library" target="_blank">Gumroadライブラリ</a>にアクセス</li>
          <li>CicadaGalleryをクリック</li>
          <li>URLまたはページ内に表示される注文IDをコピー</li>
        </ol>
      </div>
    </details>
  </div>
</div>

<style>
.form-container {
  background: #f8f9fa;
  padding: 30px;
  border-radius: 12px;
  margin: 20px 0;
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  font-weight: 600;
  margin-bottom: 8px;
  color: #333;
}

.form-group input {
  width: 100%;
  padding: 12px 16px;
  border: 2px solid #ddd;
  border-radius: 8px;
  font-size: 16px;
  transition: border-color 0.2s;
}

.form-group input:focus {
  outline: none;
  border-color: #159957;
}

.form-group small {
  display: block;
  margin-top: 6px;
  color: #666;
  font-size: 13px;
}

#submit-btn {
  width: 100%;
  padding: 14px;
  background: linear-gradient(135deg, #159957, #155799);
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

#submit-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(21, 153, 87, 0.3);
}

#submit-btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

#message {
  margin-top: 20px;
  padding: 16px;
  border-radius: 8px;
  display: none;
}

#message.show {
  display: block;
}

#message.success {
  background: #d4edda;
  color: #155724;
  border: 1px solid #c3e6cb;
}

#message.error {
  background: #f8d7da;
  color: #721c24;
  border: 1px solid #f5c6cb;
}

#message.loading {
  background: #cce5ff;
  color: #004085;
  border: 1px solid #b8daff;
}

.help-section {
  margin-top: 30px;
}

.help-section details {
  background: #fff;
  border: 1px solid #ddd;
  border-radius: 8px;
  padding: 16px;
}

.help-section summary {
  cursor: pointer;
  font-weight: 600;
  color: #155799;
}

.help-content {
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid #eee;
}

.help-content h4 {
  margin: 16px 0 8px;
  font-size: 14px;
}

.help-content ol {
  padding-left: 20px;
  margin: 0;
}

.help-content li {
  margin-bottom: 6px;
  font-size: 14px;
}

.spinner {
  display: inline-block;
  width: 16px;
  height: 16px;
  border: 2px solid #ffffff;
  border-radius: 50%;
  border-top-color: transparent;
  animation: spin 0.8s linear infinite;
  margin-right: 8px;
  vertical-align: middle;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>

<script>
const WORKER_URL = 'https://cicada-gallery-license.tetd4rthli13.workers.dev';

async function submitForm() {
  const messageDiv = document.getElementById('message');
  const submitBtn = document.getElementById('submit-btn');
  const orderId = document.getElementById('order_id').value.trim();
  const email = document.getElementById('email').value.trim();
  
  if (!orderId || !email) {
    showMessage('error', '注文IDとメールアドレスを入力してください');
    return;
  }
  
  submitBtn.disabled = true;
  submitBtn.innerHTML = '<span class="spinner"></span>処理中...';
  showMessage('loading', 'ライセンスを発行しています...');
  
  try {
    const response = await fetch(`${WORKER_URL}/issue-license`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ order_id: orderId, email: email, lang: 'ja' }),
    });
    
    const data = await response.json();
    
    if (data.success) {
      showMessage('success', '✅ ' + data.message + '<br><br>📧 メールが届かない場合は、迷惑メールフォルダもご確認ください。');
    } else {
      showMessage('error', '❌ ' + data.error);
    }
  } catch (error) {
    console.error('Error:', error);
    showMessage('error', '❌ 通信エラーが発生しました。しばらく経ってからお試しください。');
  } finally {
    submitBtn.disabled = false;
    submitBtn.innerHTML = 'ライセンスを発行';
  }
}

function showMessage(type, html) {
  const messageDiv = document.getElementById('message');
  messageDiv.className = 'show ' + type;
  messageDiv.innerHTML = html;
}

// Enter key support
document.addEventListener('DOMContentLoaded', function() {
  document.getElementById('email').addEventListener('keypress', function(e) {
    if (e.key === 'Enter') {
      submitForm();
    }
  });
});
</script>
