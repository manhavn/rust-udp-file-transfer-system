const { invoke } = window.__TAURI__.core;

const DEFAULT_UDP_PORT = "5000";
const DEFAULT_HTTP_PORT = "8080";
const DEFAULT_BLOCK_SIZE = "16384";

let statusCard;
let statusLabel;
let statusText;
let statusEndpoint;
let filePicker;
let selectedFileName;
let selectedFileMeta;
let selectedFilePath;
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
let connectionPreview;
let btnStartUpload;
let btnBgUpload;
let btnResetAdvanced;
let btnClearHistory;
let historyEmpty;
let historyList;
let historyCount;

let selectedFileSize = 0;
let directUploadRunning = false;
let backgroundUploadCount = 0;

window.addEventListener("DOMContentLoaded", () => {
  initDOMElements();
  loadSettings();
  loadHistory();
  setupEventListeners();
  updateConnectionPreview();
  updateActionAvailability();
});

function initDOMElements() {
  statusCard = document.getElementById("status-card");
  statusLabel = document.getElementById("status-label");
  statusText = document.getElementById("status-text");
  statusEndpoint = document.getElementById("status-endpoint");

  filePicker = document.getElementById("file-picker");
  selectedFileName = document.getElementById("selected-file-name");
  selectedFileMeta = document.getElementById("selected-file-meta");
  selectedFilePath = document.getElementById("selected-file-path");
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
  connectionPreview = document.getElementById("connection-preview");

  btnStartUpload = document.getElementById("btn-start-upload");
  btnBgUpload = document.getElementById("btn-bg-upload");
  btnResetAdvanced = document.getElementById("btn-reset-advanced");
  btnClearHistory = document.getElementById("btn-clear-history");
  historyEmpty = document.getElementById("history-empty");
  historyList = document.getElementById("history-list");
  historyCount = document.getElementById("history-count");
}

function setupEventListeners() {
  btnTogglePassword.addEventListener("click", () => {
    const showPassword = inputPassword.type === "password";
    inputPassword.type = showPassword ? "text" : "password";
    btnTogglePassword.textContent = showPassword ? "Ẩn" : "Hiện";
  });

  btnChooseFile.addEventListener("click", chooseFile);
  btnCreateDemo.addEventListener("click", createDemoFile);
  btnStartUpload.addEventListener("click", () => performUploadAction(false));
  btnBgUpload.addEventListener("click", () => performUploadAction(true));
  btnResetAdvanced.addEventListener("click", resetAdvancedSettings);

  btnClearHistory.addEventListener("click", () => {
    if (confirm("Xóa toàn bộ lịch sử truyền file trên máy này?")) {
      localStorage.setItem("upload_history", "[]");
      renderHistory([]);
      setStatus("info", "Đã xóa lịch sử", "Lịch sử truyền file trên máy này đã được làm sạch.");
    }
  });

  const configInputs = [inputServerIp, inputUdpPort, inputHttpPort, inputBlockSize, inputPassword];
  configInputs.forEach((input) => {
    input.addEventListener("input", () => {
      saveSettings();
      updateConnectionPreview();
      updateActionAvailability();
    });
  });

  window.addEventListener("keydown", (event) => {
    if (event.ctrlKey && event.key.toLowerCase() === "o") {
      event.preventDefault();
      chooseFile();
      return;
    }

    if (event.ctrlKey && event.key === "Enter" && !btnStartUpload.disabled) {
      event.preventDefault();
      performUploadAction(false);
    }
  });
}

async function chooseFile() {
  try {
    setStatus("busy", "Đang chọn file", "Đang mở hộp thoại chọn file từ máy tính...");
    const selectedPath = await invoke("select_file");

    if (!selectedPath) {
      setStatus("idle", "Sẵn sàng", "Đã hủy chọn file.");
      return;
    }

    await setSelectedFile(selectedPath);
    setStatus("info", "Đã chọn file", `${getFileName(selectedPath)} · ${formatFileSize(selectedFileSize)}`);
  } catch (err) {
    setStatus("error", "Không thể chọn file", String(err));
  }
}

async function createDemoFile() {
  const fileName = inputFileName.value.trim();
  const content = inputDemoContent.value;

  if (!fileName) {
    setStatus("error", "Thiếu tên file", "Nhập tên file demo trước khi tạo.");
    inputFileName.focus();
    return;
  }

  try {
    btnCreateDemo.disabled = true;
    setStatus("busy", "Đang tạo file demo", `Đang tạo ${fileName} trong thư mục tạm...`);
    const demoPath = await invoke("create_demo_file", { content, fileName });
    await setSelectedFile(demoPath);
    setStatus("success", "File demo đã sẵn sàng", `${fileName} · ${formatFileSize(selectedFileSize)}`);
  } catch (err) {
    setStatus("error", "Tạo file demo thất bại", String(err));
  } finally {
    btnCreateDemo.disabled = false;
  }
}

async function setSelectedFile(filePath) {
  inputFilePath.value = filePath;
  const fileName = getFileName(filePath);
  inputFileName.value = fileName;

  try {
    selectedFileSize = Number(await invoke("get_file_size", { filePath })) || 0;
  } catch (err) {
    selectedFileSize = 0;
    throw err;
  }

  filePicker.classList.add("has-file");
  selectedFileName.textContent = fileName;
  selectedFileMeta.textContent = formatFileSize(selectedFileSize);
  selectedFilePath.textContent = filePath;
  selectedFilePath.title = filePath;
  btnChooseFile.textContent = "Đổi file";
  updateActionAvailability();
}

function loadSettings() {
  inputServerIp.value = localStorage.getItem("server_ip") || "";
  inputUdpPort.value = localStorage.getItem("udp_port") || DEFAULT_UDP_PORT;
  inputHttpPort.value = localStorage.getItem("http_port") || DEFAULT_HTTP_PORT;
  inputBlockSize.value = localStorage.getItem("block_size") || DEFAULT_BLOCK_SIZE;
  inputPassword.value = localStorage.getItem("password") || "";
}

function resetAdvancedSettings() {
  inputUdpPort.value = DEFAULT_UDP_PORT;
  inputHttpPort.value = DEFAULT_HTTP_PORT;
  inputBlockSize.value = DEFAULT_BLOCK_SIZE;
  saveSettings();
  updateConnectionPreview();
  updateActionAvailability();
  setStatus(
    "info",
    "Đã khôi phục mặc định",
    `UDP ${DEFAULT_UDP_PORT} · HTTP ${DEFAULT_HTTP_PORT} · Block size ${DEFAULT_BLOCK_SIZE} bytes.`
  );
}

function saveSettings() {
  localStorage.setItem("server_ip", inputServerIp.value.trim());
  localStorage.setItem("udp_port", inputUdpPort.value);
  localStorage.setItem("http_port", inputHttpPort.value);
  localStorage.setItem("block_size", inputBlockSize.value);
  localStorage.setItem("password", inputPassword.value);
}

function updateConnectionPreview() {
  const serverIp = inputServerIp.value.trim();
  const udpPort = normalisePort(inputUdpPort.value, 5000);
  const httpPort = normalisePort(inputHttpPort.value, 8080);
  const endpoint = serverIp ? `${serverIp} · UDP ${udpPort} · HTTP ${httpPort}` : "Chưa cấu hình";

  connectionPreview.textContent = endpoint;
  connectionPreview.title = endpoint;
  statusEndpoint.textContent = serverIp ? `${serverIp}:${udpPort}` : "Chưa cấu hình server";
  statusEndpoint.title = serverIp ? `UDP ${serverIp}:${udpPort} · HTTP ${serverIp}:${httpPort}` : "";
}

function updateActionAvailability() {
  const hasFile = Boolean(inputFilePath.value);
  const hasServer = Boolean(inputServerIp.value.trim());
  const portsValid = isValidPort(inputUdpPort.value) && isValidPort(inputHttpPort.value);
  const blockSizeValid = Number(inputBlockSize.value) >= 512;
  const ready = hasFile && hasServer && portsValid && blockSizeValid;

  btnStartUpload.disabled = !ready || directUploadRunning;
  btnBgUpload.disabled = !ready || directUploadRunning;
  if (btnResetAdvanced) {
    btnResetAdvanced.disabled = directUploadRunning;
  }
}

function loadHistory() {
  const historyJson = localStorage.getItem("upload_history") || "[]";
  let history = [];

  try {
    history = JSON.parse(historyJson);
  } catch (_) {
    history = [];
  }

  renderHistory(history);
}

function saveHistoryItem(fileName, filePath, fileSize, sha256, isSuccess, statusMsg) {
  const historyJson = localStorage.getItem("upload_history") || "[]";
  let history = [];

  try {
    history = JSON.parse(historyJson);
  } catch (_) {
    history = [];
  }

  history.unshift({
    fileName,
    filePath,
    fileSize,
    timestamp: Date.now(),
    sha256,
    isSuccess,
    statusMsg
  });

  history = history.slice(0, 50);
  localStorage.setItem("upload_history", JSON.stringify(history));
  renderHistory(history);
}

function renderHistory(history) {
  historyCount.textContent = history.length;
  const isEmpty = history.length === 0;
  historyEmpty.hidden = !isEmpty;
  historyList.hidden = isEmpty;
  btnClearHistory.hidden = isEmpty;

  if (isEmpty) {
    historyList.replaceChildren();
    return;
  }

  const fragment = document.createDocumentFragment();

  history.forEach((item) => {
    const itemEl = document.createElement("article");
    itemEl.className = "history-item";

    const main = document.createElement("div");
    main.className = "history-main";

    const titleRow = document.createElement("div");
    titleRow.className = "history-title-row";

    const fileName = document.createElement("span");
    fileName.className = "history-filename";
    fileName.textContent = item.fileName || "Không rõ tên file";
    fileName.title = item.filePath || item.fileName || "";

    const badge = document.createElement("span");
    badge.className = `history-status-badge ${item.isSuccess ? "success" : "error"}`;
    badge.textContent = item.isSuccess ? "Thành công" : "Lỗi";

    titleRow.append(fileName, badge);

    const metaRow = document.createElement("div");
    metaRow.className = "history-meta-row";
    metaRow.append(
      textSpan(formatFileSize(Number(item.fileSize) || 0)),
      textSpan("·", "separator"),
      textSpan(formatTimestamp(item.timestamp))
    );

    const message = document.createElement("p");
    message.className = "history-message";
    message.textContent = item.statusMsg || (item.isSuccess ? "Hoàn tất" : "Thất bại");

    const hashRow = document.createElement("div");
    hashRow.className = "history-hash";
    const hashLabel = document.createElement("span");
    hashLabel.textContent = "SHA-256";
    const hashValue = document.createElement("code");
    hashValue.textContent = item.sha256 || "N/A";
    hashValue.title = item.sha256 || "N/A";
    hashRow.append(hashLabel, hashValue);

    main.append(titleRow, metaRow, message, hashRow);

    const actions = document.createElement("div");
    actions.className = "history-actions";

    const reuseButton = document.createElement("button");
    reuseButton.type = "button";
    reuseButton.className = "mini-button accent";
    reuseButton.textContent = "Dùng lại";
    reuseButton.disabled = !item.filePath;
    reuseButton.addEventListener("click", async () => {
      try {
        await setSelectedFile(item.filePath);
        setStatus("info", "Đã chọn lại file", `${item.fileName} · ${formatFileSize(selectedFileSize)}`);
        window.scrollTo({ top: 0, behavior: "smooth" });
      } catch (err) {
        setStatus("error", "Không thể dùng lại file", "File có thể đã bị di chuyển hoặc xóa khỏi máy.");
      }
    });

    const copyButton = document.createElement("button");
    copyButton.type = "button";
    copyButton.className = "mini-button";
    copyButton.textContent = "Copy hash";
    copyButton.disabled = !item.sha256 || item.sha256 === "N/A";
    copyButton.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(item.sha256);
        const originalText = copyButton.textContent;
        copyButton.textContent = "Đã copy";
        window.setTimeout(() => {
          copyButton.textContent = originalText;
        }, 1300);
      } catch (_) {
        setStatus("error", "Không thể sao chép", "Clipboard hiện không khả dụng.");
      }
    });

    actions.append(reuseButton, copyButton);
    itemEl.append(main, actions);
    fragment.appendChild(itemEl);
  });

  historyList.replaceChildren(fragment);
}

async function performUploadAction(isBackground) {
  const filePath = inputFilePath.value;
  const serverIp = inputServerIp.value.trim();
  const udpPort = normalisePort(inputUdpPort.value, 5000);
  const httpPort = normalisePort(inputHttpPort.value, 8080);
  const blockSize = Math.max(512, parseInt(inputBlockSize.value, 10) || 16384);
  const password = inputPassword.value;

  if (!filePath) {
    setStatus("error", "Chưa có file", "Chọn hoặc tạo một file trước khi gửi.");
    return;
  }

  if (!serverIp) {
    setStatus("error", "Chưa có server", "Nhập địa chỉ server nhận file.");
    inputServerIp.focus();
    return;
  }

  if (!isValidPort(udpPort) || !isValidPort(httpPort)) {
    setStatus("error", "Cổng không hợp lệ", "UDP/HTTP port phải nằm trong khoảng 1–65535.");
    return;
  }

  saveSettings();
  updateConnectionPreview();

  const fileName = getFileName(filePath);
  let fileSize = selectedFileSize;
  if (!fileSize) {
    try {
      fileSize = Number(await invoke("get_file_size", { filePath })) || 0;
    } catch (_) {
      fileSize = 0;
    }
  }

  if (isBackground) {
    backgroundUploadCount += 1;
    setStatus("busy", "Đang gửi nền", `${fileName} đang được xử lý. Bạn có thể tiếp tục thao tác trong ứng dụng.`);
    calculateAndUpload(filePath, serverIp, udpPort, httpPort, blockSize, password, fileName, fileSize, true)
      .finally(() => {
        backgroundUploadCount = Math.max(0, backgroundUploadCount - 1);
      });
    return;
  }

  directUploadRunning = true;
  updateActionAvailability();
  setStatus("busy", "Đang chuẩn bị truyền", `Đang tính SHA-256 cho ${fileName}...`);

  try {
    await calculateAndUpload(filePath, serverIp, udpPort, httpPort, blockSize, password, fileName, fileSize, false);
  } finally {
    directUploadRunning = false;
    updateActionAvailability();
  }
}

async function calculateAndUpload(filePath, serverIp, udpPort, httpPort, blockSize, password, fileName, fileSize, isBackground) {
  let fileHash = "N/A";

  try {
    fileHash = await invoke("calculate_hash", { filePath });
  } catch (err) {
    const errorMsg = `Lỗi tính hash: ${err}`;
    setStatus("error", "Không thể tính SHA-256", String(err));
    saveHistoryItem(fileName, filePath, fileSize, fileHash, false, errorMsg);
    return;
  }

  setStatus("busy", isBackground ? "Đang gửi nền" : "Đang truyền file", `${fileName} → ${serverIp}:${udpPort}`);

  try {
    const code = await invoke("perform_upload", {
      filePath,
      serverIp,
      udpPort,
      httpPort,
      blockSize,
      password: password || null
    });

    const isSuccess = code === 0;
    const errorDescription = getErrorMessage(code);
    const historyPrefix = isBackground ? "[Gửi nền] " : "";
    const statusMsg = isSuccess
      ? `${historyPrefix}Hoàn tất (mã 0)`
      : `${historyPrefix}Lỗi ${code}: ${errorDescription}`;

    if (isSuccess) {
      setStatus(
        "success",
        isBackground ? "Gửi nền hoàn tất" : "Truyền file hoàn tất",
        `${fileName} · ${formatFileSize(fileSize)} · SHA-256 ${shortHash(fileHash)}`
      );
    } else {
      setStatus("error", "Truyền file thất bại", `Mã ${code}: ${errorDescription}`);
    }

    saveHistoryItem(fileName, filePath, fileSize, fileHash, isSuccess, statusMsg);
  } catch (err) {
    const crashMsg = `Lỗi kết nối/native: ${err}`;
    setStatus("error", "Không thể hoàn tất truyền file", String(err));
    saveHistoryItem(fileName, filePath, fileSize, fileHash, false, crashMsg);
  }
}

function setStatus(state, label, message) {
  statusCard.dataset.state = state;
  statusLabel.textContent = label;
  statusText.textContent = message;
}

function getFileName(filePath) {
  return String(filePath).split(/[/\\]/).pop() || "Không rõ tên file";
}

function shortHash(hash) {
  if (!hash || hash === "N/A") return "N/A";
  return hash.length > 18 ? `${hash.slice(0, 10)}…${hash.slice(-6)}` : hash;
}

function formatFileSize(bytes) {
  const value = Number(bytes) || 0;
  if (value <= 0) return "0 B";

  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  const amount = value / Math.pow(1024, index);
  return `${amount >= 100 || index === 0 ? amount.toFixed(0) : amount.toFixed(2).replace(/\.00$/, "")} ${units[index]}`;
}

function formatTimestamp(timestamp) {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return "Không rõ thời gian";

  return new Intl.DateTimeFormat("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit"
  }).format(date);
}

function normalisePort(value, fallback) {
  const parsed = parseInt(value, 10);
  return Number.isInteger(parsed) ? parsed : fallback;
}

function isValidPort(value) {
  const port = Number(value);
  return Number.isInteger(port) && port >= 1 && port <= 65535;
}

function textSpan(text, className = "") {
  const span = document.createElement("span");
  span.textContent = text;
  if (className) span.className = className;
  return span;
}

function getErrorMessage(code) {
  switch (code) {
    case -1: return "Tham số không hợp lệ";
    case -2: return "Không tìm thấy hoặc không đọc được file";
    case -3: return "Tính mã băm SHA-256 thất bại";
    case -4: return "Đăng ký tải lên qua HTTP thất bại";
    case -5: return "Không phân giải được địa chỉ IP/UDP";
    case -6: return "Không thể bind cổng UDP cục bộ";
    case -7: return "Truyền UDP thất bại hoặc hết thời gian chờ";
    case -99: return "Native FFI gặp lỗi";
    default: return `Mã lỗi không xác định: ${code}`;
  }
}
