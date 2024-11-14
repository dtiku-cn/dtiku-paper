"use client"

import { ajax } from "../ajax"
import { Button, Drawer, Table } from "antd";
import { useEffect, useState } from "react";
import { EditOutlined } from "@ant-design/icons"
import SvelteJSONEditor from "../../component/JsonEditor"

const { Column } = Table;

export default function ConfigList() {
    const [configs, setConfigs] = useState([]);
    const [editKey, setEditKey] = useState("");
    const [content, setContent] = useState({ json: null });
    useEffect(() => {
        (async () => {
            let { data: configs, total } = await ajax("/configs")
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
    const open = !!editKey;
    return (
        <>
            <Table dataSource={configs}>
                <Column title="id" dataIndex="id" key="id" />
                <Column title="key" dataIndex="key" key="key" />
                <Column title="key_desc" dataIndex="key_desc" key="key_desc" />
                <Column title="modified" dataIndex="modified" key="modified" />
                <Column title="actions" dataIndex="key" render={renderEditor} />
            </Table>
            <Drawer title={editKey} onClose={handleClose} open={open}>
                <SvelteJSONEditor content={content} mode='text' onChange={setContent} />
            </Drawer>
        </>
    )
}
