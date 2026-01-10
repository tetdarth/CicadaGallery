---
layout: default
title: License Activation
lang: en
---

[🇯🇵 日本語](../license.md) | **🇺🇸 English**

# 🎫 License Activation

If you purchased through Gumroad, you can obtain your license key using the form below.

<div id="license-app">
  <div class="form-container">
    <div class="form-group">
      <label for="order_id">Order ID</label>
      <input type="text" id="order_id" placeholder="e.g. XXXXXXXXXX" required>
      <small>Found in your Gumroad purchase confirmation email</small>
    </div>
    
    <div class="form-group">
      <label for="email">Email Address</label>
      <input type="email" id="email" placeholder="Email used for purchase" required>
    </div>
    
    <button id="submit-btn" onclick="submitForm()">Issue License</button>
    
    <div id="message"></div>
  </div>
  
  <div class="help-section">
    <details>
      <summary>❓ How to find your Order ID</summary>
      <div class="help-content">
        <h4>📧 Check your Email</h4>
        <ol>
          <li>Open the "Receipt for your purchase" email from Gumroad</li>
          <li>Copy the Order ID from the email</li>
        </ol>
        
        <h4>📚 Check Gumroad Library</h4>
        <ol>
          <li>Visit <a href="https://app.gumroad.com/library" target="_blank">Gumroad Library</a></li>
          <li>Click on CicadaGallery</li>
          <li>Copy the Order ID from the URL or page</li>
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
    showMessage('error', 'Please enter both Order ID and email address');
    return;
  }
  
  submitBtn.disabled = true;
  submitBtn.innerHTML = '<span class="spinner"></span>Processing...';
  showMessage('loading', 'Issuing your license...');
  
  try {
    const response = await fetch(`${WORKER_URL}/issue-license`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ order_id: orderId, email: email }),
    });
    
    const data = await response.json();
    
    if (data.success) {
      showMessage('success', '✅ ' + data.message + '<br><br>📧 If you don\'t receive the email, please check your spam folder.');
    } else {
      showMessage('error', '❌ ' + data.error);
    }
  } catch (error) {
    console.error('Error:', error);
    showMessage('error', '❌ A network error occurred. Please try again later.');
  } finally {
    submitBtn.disabled = false;
    submitBtn.innerHTML = 'Issue License';
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
