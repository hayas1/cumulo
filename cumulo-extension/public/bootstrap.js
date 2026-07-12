// trunk の inline ローダを使わず、外部 module として wasm を初期化する（MV3 CSP 対応）。
// bin の fn main が start として init 中に走り body へマウントするので、init を呼ぶだけでよい。
import init from "./cumulo-extension.js";

await init();
