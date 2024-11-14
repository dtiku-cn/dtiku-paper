"use client"

import { get, put } from "../ajax"
import { Button, Drawer, message, Table } from "antd";
import { useEffect, useState } from "react";
import { EditOutlined } from "@ant-design/icons"
import SvelteJSONEditor from "../../component/JsonEditor"
import { Content, JSONContent, TextContent } from "vanilla-jsoneditor";

const { Column } = Table;

export default function ConfigList() {
    const [configs, setConfigs] = useState([]);
    const [editKey, setEditKey] = useState("");
    const [content, setContent] = useState({} as Content);
    const [messageApi, contextHolder] = message.useMessage();
    useEffect(() => {
        (async () => {
            let { data: configs, total } = await get("/configs")
            console.log('config-total', total)
            setConfigs(configs);
        })()
    }, []);
    const renderEditor = (key: string, { value }: { value: string }) => {
        return <Button type="link" icon={<EditOutlined />} onClick={() => {
            setEditKey(key);
            setContent({ json: JSON.parse(value) });
        }} />
    }
    const handleClose = () => {
        setEditKey('')
    }
    const handleSubmit = async () => {
        const body = 'json' in content ? JSON.stringify(content.json) : content.text;
        await put(`/configs/${editKey}`, { headers: { "Content-Type": "application/json" }, body })
        messageApi.success("保存成功");
        setEditKey('')
    };
    const open = !!editKey;
    return (
        <>
            {contextHolder}
            <Table dataSource={configs}>
                <Column title="id" dataIndex="id" key="id" />
                <Column title="key" dataIndex="key" key="key" />
                <Column title="key_desc" dataIndex="key_desc" key="key_desc" />
                <Column title="modified" dataIndex="modified" key="modified" />
                <Column title="actions" dataIndex="key" render={renderEditor} />
            </Table>
            <Drawer title={editKey} width="600" onClose={handleClose} open={open}
                extra={<Button type="primary" onClick={handleSubmit}>提交</Button>}>
                <SvelteJSONEditor style={{ height: '100%' }} mode='text' content={content} onChange={setContent} />
            </Drawer>
        </>
    )
}
