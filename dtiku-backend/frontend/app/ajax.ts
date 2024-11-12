export async function ajax(url: string) {
    let resp = await fetch(`/api/${url}`);
    return await resp.json();
}