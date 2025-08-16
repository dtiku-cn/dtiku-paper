(async function () {
    // 拒绝无头浏览器的请求
    if (navigator.webdriver)
        return;
    let agent = navigator.userAgent;
    if (/headless/i.test(agent))
        return;
    let appVersion = navigator.appVersion;
    if (/headless/i.test(appVersion))
        return;
    let correctPrototypes = PluginArray.prototype === navigator.plugins.__proto__
        & MimeTypeArray.prototype === navigator.mimeTypes.__proto__;
    if (navigator.plugins.length > 0)
        correctPrototypes &= Plugin.prototype === navigator.plugins[0].__proto__;
    if (navigator.mimeTypes.length > 0)
        correctPrototypes &= MimeType.prototype === navigator.mimeTypes[0].__proto__;
    if (!correctPrototypes)
        return;
    if (!navigator.language || navigator.languages.length === 0)
        return;

    // 从 meta 标签读取 secret
    const meta = document.querySelector('meta[name="anti-bot-token-secret"]');
    const secret = meta ? meta.content : 'anti-bot-default-secret';

    // 获取时间戳
    const now_week = Math.floor(Date.now() / 1000 / 60 / 60 / 24 / 7);

    // 生成浏览器ID
    let visitorId = localStorage.getItem("anti-fp-id");
    if (!visitorId) {
        visitorId = crypto.randomUUID(); // 生成稳定 UUID
    }

    // 生成 SHA256 token
    const msg = `${now_week}${visitorId}${secret}`;
    const buf = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(msg));
    const hashArray = Array.from(new Uint8Array(buf));
    const token = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');

    // 写入 Cookie
    document.cookie = `x-anti-token=${token}; path=/; max-age=604800; Secure; SameSite=Strict`;
    document.cookie = `x-fp=${visitorId}; path=/; max-age=604800; Secure; SameSite=Strict`;

    window.location.reload();
})();