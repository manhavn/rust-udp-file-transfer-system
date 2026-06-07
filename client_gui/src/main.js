const { invoke } = window.__TAURI__.core;

// DOM Elements
let statusText;
let inputFileName;
let btnChooseFile;
let inputDemoContent;
let btnCreateDemo;
let inputFilePath;

let inputServerIp;
let inputUdpPort;
let inputHttpPort;
let inputBlockSize;
let inputPassword;
let btnTogglePassword;

let btnStartUpload;
let btnBgUpload;
let btnClearHistory;
let historyEmpty;
let historyList;
let historyCount;

// Load config and history on load
window.addEventListener("DOMContentLoaded", () => {
  initDOMElements();
  loadSettings();
  loadHistory();
  setupEventListeners();
});

function initDOMElements() {
  statusText = document.getElementById("status-text");
  inputFileName = document.getElementById("input-file-name");
  btnChooseFile = document.getElementById("btn-choose-file");
  inputDemoContent = document.getElementById("input-demo-content");
  btnCreateDemo = document.getElementById("btn-create-demo");
  inputFilePath = document.getElementById("input-file-path");

  inputServerIp = document.getElementById("input-server-ip");
  inputUdpPort = document.getElementById("input-udp-port");
  inputHttpPort = document.getElementById("input-http-port");
  inputBlockSize = document.getElementById("input-block-size");
  inputPassword = document.getElementById("input-password");
  btnTogglePassword = document.getElementById("btn-toggle-password");

  btnStartUpload = document.getElementById("btn-start-upload");
  btnBgUpload = document.getElementById("btn-bg-upload");
  btnClearHistory = document.getElementById("btn-clear-history");
  historyEmpty = document.getElementById("history-empty");
  historyList = document.getElementById("history-list");
  historyCount = document.getElementById("history-count");
}

function setupEventListeners() {
  // Toggle password visibility
  btnTogglePassword.addEventListener("click", () => {
    if (inputPassword.type === "password") {
      inputPassword.type = "text";
      btnTogglePassword.textContent = "Ẩn";
    } else {
      inputPassword.type = "password";
      btnTogglePassword.textContent = "Hiện";
    }
  });

  // Choose file dialog
  btnChooseFile.addEventListener("click", async () => {
    try {
      statusText.textContent = "Đang mở hộp thoại chọn file...";
      const selectedPath = await invoke("select_file");
      if (selectedPath) {
        inputFilePath.value = selectedPath;
        // Extract filename from path
        const fileName = selectedPath.split(/[/\\]/).pop();
        inputFileName.value = fileName;

        // Get file size
        const size = await invoke("get_file_size", { filePath: selectedPath });
        statusText.textContent = `Đã chọn file: ${fileName} (${formatFileSize(size)})`;
      } else {
        statusText.textContent = "Hủy chọn file.";
      }
    } catch (err) {
      statusText.textContent = `Lỗi chọn file: ${err}`;
    }
  });

  // Create demo file
  btnCreateDemo.addEventListener("click", async () => {
    const fileName = inputFileName.value.trim();
    const content = inputDemoContent.value;
    if (!fileName) {
      alert("Vui lòng nhập tên file demo trước");
      return;
    }

    try {
      statusText.textContent = "Đang tạo file demo...";
      const demoPath = await invoke("create_demo_file", { content, fileName });
      inputFilePath.value = demoPath;
      
      const size = await invoke("get_file_size", { filePath: demoPath });
      statusText.textContent = `Đã tạo file demo thành công: ${fileName} (${formatFileSize(size)})`;
    } catch (err) {
      statusText.textContent = `Tạo file demo lỗi: ${err}`;
    }
  });

  // Start direct upload
  btnStartUpload.addEventListener("click", () => {
    performUploadAction(false);
  });

  // Start background upload
  btnBgUpload.addEventListener("click", () => {
    performUploadAction(true);
  });

  // Clear history
  btnClearHistory.addEventListener("click", () => {
    if (confirm("Bạn có chắc chắn muốn xóa toàn bộ lịch sử tải lên?")) {
      localStorage.setItem("upload_history", "[]");
      renderHistory([]);
    }
  });

  // Save configurations when inputs change
  const configInputs = [inputServerIp, inputUdpPort, inputHttpPort, inputBlockSize, inputPassword];
  configInputs.forEach(input => {
    input.addEventListener("input", saveSettings);
  });
}

// Settings management
function loadSettings() {
  inputServerIp.value = localStorage.getItem("server_ip") || "";
  inputUdpPort.value = localStorage.getItem("udp_port") || "5000";
  inputHttpPort.value = localStorage.getItem("http_port") || "8080";
  inputBlockSize.value = localStorage.getItem("block_size") || "16384";
  inputPassword.value = localStorage.getItem("password") || "";
}

function saveSettings() {
  localStorage.setItem("server_ip", inputServerIp.value);
  localStorage.setItem("udp_port", inputUdpPort.value);
  localStorage.setItem("http_port", inputHttpPort.value);
  localStorage.setItem("block_size", inputBlockSize.value);
  localStorage.setItem("password", inputPassword.value);
}

// History management
function loadHistory() {
  const historyJson = localStorage.getItem("upload_history") || "[]";
  let history = [];
  try {
    history = JSON.parse(historyJson);
  } catch (e) {
    history = [];
  }
  renderHistory(history);
}

function saveHistoryItem(fileName, filePath, fileSize, sha256, isSuccess, statusMsg) {
  const historyJson = localStorage.getItem("upload_history") || "[]";
  let history = [];
  try {
    history = JSON.parse(historyJson);
  } catch (e) {
    history = [];
  }

  const newItem = {
    fileName,
    filePath,
    fileSize,
    timestamp: Date.now(),
    sha256,
    isSuccess,
    statusMsg
  };

  // Prepend to list, limit to 50 items
  history.unshift(newItem);
  history = history.slice(0, 50);

  localStorage.setItem("upload_history", JSON.stringify(history));
  renderHistory(history);
}

function renderHistory(history) {
  historyCount.textContent = history.length;
  
  if (history.length === 0) {
    historyEmpty.style.display = "block";
    historyList.style.display = "none";
    btnClearHistory.style.display = "none";
    return;
  }

  historyEmpty.style.display = "none";
  historyList.style.display = "flex";
  btnClearHistory.style.display = "block";

  historyList.innerHTML = "";
  history.forEach(item => {
    const itemEl = document.createElement("div");
    itemEl.className = `history-item ${item.isSuccess ? 'success' : 'error'}`;

    const timestampStr = formatTimestamp(item.timestamp);
    const sizeStr = formatFileSize(item.fileSize);

    itemEl.innerHTML = `
      <div class="history-row">
        <span class="history-filename">${item.fileName}</span>
        <span class="history-size">${sizeStr}</span>
      </div>
      <div class="history-meta">Thời gian: ${timestampStr}</div>
      <div class="history-status">Trạng thái: ${item.statusMsg}</div>
      <div class="history-hash-box">
        <div class="hash-details">
          <span class="hash-label">Hash ID (Khớp Backend):</span>
          <span class="hash-value monospace">${item.sha256}</span>
        </div>
        <button class="btn-copy" data-hash="${item.sha256}">SAO CHÉP</button>
      </div>
    `;

    // Copy event handler
    itemEl.querySelector(".btn-copy").addEventListener("click", (e) => {
      const hash = e.target.getAttribute("data-hash");
      navigator.clipboard.writeText(hash).then(() => {
        const originalText = e.target.textContent;
        e.target.textContent = "ĐÃ CHÉP!";
        e.target.style.color = "#34d399";
        setTimeout(() => {
          e.target.textContent = originalText;
          e.target.style.color = "";
        }, 1500);
      });
    });

    historyList.appendChild(itemEl);
  });
}

// Helpers
function formatFileSize(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatTimestamp(timestamp) {
  const date = new Date(timestamp);
  const pad = (num) => String(num).padStart(2, '0');
  const day = pad(date.getDate());
  const month = pad(date.getMonth() + 1);
  const year = date.getFullYear();
  const hours = pad(date.getHours());
  const minutes = pad(date.getMinutes());
  const seconds = pad(date.getSeconds());
  return `${day}/${month}/${year} ${hours}:${minutes}:${seconds}`;
}

// Core Upload Action
async function performUploadAction(isBackground) {
  const filePath = inputFilePath.value;
  const serverIp = inputServerIp.value.trim();
  const udpPort = parseInt(inputUdpPort.value) || 5000;
  const httpPort = parseInt(inputHttpPort.value) || 8080;
  const blockSize = parseInt(inputBlockSize.value) || 16384;
  const password = inputPassword.value;

  if (!filePath) {
    alert("Vui lòng chọn hoặc tạo file trước");
    return;
  }
  if (!serverIp) {
    alert("Vui lòng nhập địa chỉ IP Server");
    return;
  }

  saveSettings();

  const fileName = filePath.split(/[/\\]/).pop();
  let fileSize = 0;
  try {
    fileSize = await invoke("get_file_size", { filePath });
  } catch (e) {
    console.error("Lỗi lấy kích thước file:", e);
  }

  if (isBackground) {
    statusText.textContent = "Đã đẩy tác vụ tải lên chạy ngầm. Bạn có thể tiếp tục thao tác.";
    
    // Run asynchronous chain
    calculateAndUpload(filePath, serverIp, udpPort, httpPort, blockSize, password, fileName, fileSize, true);
  } else {
    statusText.textContent = "Đang tính toán hash và bắt đầu tải lên...";
    
    // Disable action buttons during direct upload
    btnStartUpload.disabled = true;
    btnBgUpload.disabled = true;
    
    calculateAndUpload(filePath, serverIp, udpPort, httpPort, blockSize, password, fileName, fileSize, false)
      .finally(() => {
        btnStartUpload.disabled = false;
        btnBgUpload.disabled = false;
      });
  }
}

async function calculateAndUpload(filePath, serverIp, udpPort, httpPort, blockSize, password, fileName, fileSize, isBg) {
  let fileHash = "N/A";
  try {
    fileHash = await invoke("calculate_hash", { filePath });
  } catch (err) {
    const errorMsg = `Lỗi tính hash: ${err}`;
    if (!isBg) {
      statusText.textContent = errorMsg;
    }
    saveHistoryItem(fileName, filePath, fileSize, fileHash, false, errorMsg);
    return;
  }

  try {
    const code = await invoke("perform_upload", {
      filePath,
      serverIp,
      udpPort,
      httpPort,
      blockSize,
      password: password || null
    });

    const isSuccess = (code === 0);
    const errorDescription = getErrorMessage(code);
    const statusMsg = isSuccess ? "Thành công (Mã 0)" : `Lỗi (${code}): ${errorDescription}`;

    if (isSuccess) {
      const successMsg = `Thành công: Tải lên hoàn tất (Mã 0)!\nHash ID: ${fileHash}`;
      if (!isBg) {
        statusText.textContent = successMsg;
      }
    } else {
      const failMsg = `Thất bại: Lỗi (${code}) - ${errorDescription}`;
      if (!isBg) {
        statusText.textContent = failMsg;
      }
    }

    saveHistoryItem(fileName, filePath, fileSize, fileHash, isSuccess, isBg ? `[Chạy ngầm] ${statusMsg}` : statusMsg);
  } catch (err) {
    const crashMsg = `Native crash / Lỗi kết nối: ${err}`;
    if (!isBg) {
      statusText.textContent = crashMsg;
    }
    saveHistoryItem(fileName, filePath, fileSize, fileHash, false, crashMsg);
  }
}

function getErrorMessage(code) {
  switch (code) {
    case -1: return "Tham số không hợp lệ";
    case -2: return "Không tìm thấy/không đọc được file";
    case -3: return "Tính mã băm (hash) thất bại";
    case -4: return "Đăng ký tải lên qua HTTP thất bại";
    case -5: return "Không phân giải được địa chỉ IP/UDP";
    case -6: return "Lỗi bind cổng UDP cục bộ";
    case -7: return "Truyền tải UDP thất bại hoặc quá hạn (Timeout)";
    case -99: return "Native FFI crash hoặc JNA lỗi";
    default: return `Mã lỗi không xác định: ${code}`;
  }
}
