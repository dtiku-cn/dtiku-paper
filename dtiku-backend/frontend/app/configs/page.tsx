"use client"

import { ajax } from "../ajax"
import { Button, Table } from "antd";
import { useEffect, useState } from "react";

const { Column } = Table;

export default function ConfigList() {
    const [configs, setConfigs] = useState([])
    useEffect(() => {
        (async () => {
            let { data: configs, total } = await ajax("/configs")
            console.log('config-total', total)
            setConfigs(configs);
        })()
    },[]);
    return (
        <Table dataSource={configs}>
            <Column title="id" dataIndex="id" key="id" />
            <Column title="key" dataIndex="key" key="key" />
            <Column title="key_desc" dataIndex="key_desc" key="key_desc" />
        </Table>
    )
}
