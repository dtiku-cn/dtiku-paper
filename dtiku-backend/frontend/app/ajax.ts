export async function ajax(url: string, options?: RequestInit) {
    let resp = await fetch(`/api/${url}`, options);
    return await resp.json();
}

export async function get(url: string, options?: RequestInit) {
    return await ajax(url, { method: "GET", ...options });
}

export async function post(url: string, options?: RequestInit) {
    return await ajax(url, { method: "POST", ...options });
}

export async function put(url: string, options?: RequestInit) {
    return await ajax(url, { method: "PUT", ...options });
}