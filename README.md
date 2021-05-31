# mock-yo-stream

Rust與串流練習

### 功能

- 利用RTMP協定進行串流(OBS串流成功)
- 利用HLS協定進行播放(網頁播放成功)
- 利用websocket協定即時通訊(網頁通訊成功)
- 將串流影像儲存成ts檔(不含m3u8)

### 已註解的功能

將串流影像儲存成flv檔，執行程式時開頭會警告相關變數或函式從未使用

### 其他

- 串流影像儲存在專案資料夾底下的`video資料夾`
- 每次收到串流請求時都會將video資料夾清空
- ts檔命名依照當下串流時長
- 最後一個ts檔名為`0.ts`
- 將影像儲存成flv的功能會持續將串流影像存放在記憶體，直到串流結束後再寫成單一檔案

### 執行
```
cargo run
```
