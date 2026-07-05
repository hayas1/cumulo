//! フォーマット共通のエラー。de/ser 双方の serde 機構が `custom` 経由で生成する。

/// ドットパス・クエリのフォーマット固有エラー。中身は最小の文字列メッセージ。
#[derive(Debug)]
pub struct Error(String);

impl Error {
    /// このフォーマットが扱えない種別（seq・深いネスト・bytes 等）を弾くエラー。
    /// 文字列スカラと 1 段 map 以外は載らないので、各 Serializer が該当メソッドでこれを返す。
    pub fn unsupported<T>(what: &str) -> Result<T, Self> {
        Err(Error(format!("query format cannot serialize {what}")))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}
