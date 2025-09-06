use chrono::Local;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use reqwest::blocking::Client;
use serde_json::json;
use std::collections::BTreeMap;

/// 支付宝服务
pub struct AlipayService {
    api_url: String,
    app_id: String,
    notify_url: String,
    rsa_private_key: String,
    total_fee: String,
    out_trade_no: String,
    subject: String,
    charset: String,
}

impl AlipayService {
    pub fn new(api_url: &str, app_id: &str, notify_url: &str, rsa_private_key: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
            app_id: app_id.to_string(),
            notify_url: notify_url.to_string(),
            rsa_private_key: rsa_private_key.to_string(),
            total_fee: "0.00".to_string(),
            out_trade_no: "".to_string(),
            subject: "".to_string(),
            charset: "utf-8".to_string(),
        }
    }

    pub fn set_total_fee(mut self, amount: f64) -> Self {
        self.total_fee = format!("{:.2}", amount);
        self
    }

    pub fn set_out_trade_no(mut self, out_trade_no: &str) -> Self {
        self.out_trade_no = out_trade_no.to_string();
        self
    }

    pub fn set_subject(mut self, subject: &str) -> Self {
        self.subject = subject.to_string();
        self
    }

    /// 生成待签名字符串
    fn get_sign_content(&self, params: &BTreeMap<String, String>) -> String {
        params
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// RSA2 签名
    fn sign(&self, data: &str) -> String {
        let pem = format!(
            "-----BEGIN RSA PRIVATE KEY-----\n{}\n-----END RSA PRIVATE KEY-----",
            self.rsa_private_key
                .as_bytes()
                .chunks(64)
                .map(|c| std::str::from_utf8(c).unwrap())
                .collect::<Vec<_>>()
                .join("\n")
        );

        let key = PKey::from_rsa(Rsa::private_key_from_pem(pem.as_bytes()).unwrap()).unwrap();
        let mut signer = Signer::new(openssl::hash::MessageDigest::sha256(), &key).unwrap();
        signer.update(data.as_bytes()).unwrap();
        base64::encode(signer.sign_to_vec().unwrap())
    }

    /// 发起支付
    pub fn do_pay(&self) -> serde_json::Value {
        // 业务参数
        let biz_content = json!({
            "out_trade_no": self.out_trade_no,
            "total_amount": self.total_fee,
            "subject": self.subject,
            "timeout_express": "2h",
        });

        // 公共参数
        let mut params = BTreeMap::new();
        params.insert("app_id".into(), self.app_id.clone());
        params.insert("method".into(), "alipay.trade.precreate".into());
        params.insert("format".into(), "JSON".into());
        params.insert("charset".into(), self.charset.clone());
        params.insert("sign_type".into(), "RSA2".into());
        params.insert(
            "timestamp".into(),
            Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        );
        params.insert("version".into(), "1.0".into());
        params.insert("notify_url".into(), self.notify_url.clone());
        params.insert("biz_content".into(), biz_content.to_string());

        // 签名
        let sign_content = self.get_sign_content(&params);
        let sign = self.sign(&sign_content);
        params.insert("sign".into(), sign);

        // 发送请求
        let client = Client::new();
        let resp = client
            .post(self.api_url.to_string())
            .form(&params)
            .send()
            .unwrap()
            .text()
            .unwrap();

        serde_json::from_str(&resp).unwrap()
    }
}
