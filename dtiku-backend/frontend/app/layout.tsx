"use client"

import { Layout, Menu, MenuProps } from 'antd'
import './globals.css'
import { useEffect, useState } from 'react'
import {
  DesktopOutlined,
  FileOutlined,
  PieChartOutlined,
  TeamOutlined,
  UserOutlined,
} from '@ant-design/icons'
import Link from 'next/link'
import { AntdRegistry } from '@ant-design/nextjs-registry';

const { Content, Footer, Sider } = Layout;

type MenuItem = Required<MenuProps>['items'][number];

function getItem(
  label: React.ReactNode,
  key: React.Key,
  icon?: React.ReactNode,
  children?: MenuItem[],
): MenuItem {
  return {
    key,
    icon,
    children,
    label,
  } as MenuItem;
}

const items: MenuItem[] = [
  getItem(<Link href="/">Dashboard</Link>, '/', <PieChartOutlined />),
  getItem(<Link href="/configs">Config</Link>, '/configs', <DesktopOutlined />),
  getItem(<Link href="/tasks">Task</Link>, '/tasks', <UserOutlined />),
  getItem('Team', 'sub2', <TeamOutlined />, [getItem('Team 1', '6'), getItem('Team 2', '8')]),
  getItem('Files', '9', <FileOutlined />),
];

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const [selectedKey, setSelectedKey] = useState("");
  const [collapsed, setCollapsed] = useState(false);
  useEffect(() => {
    setSelectedKey(location.pathname);
  }, [location.pathname]);
  return (
    <html lang="en">
      <body>
        <AntdRegistry>
          <Layout style={{ minHeight: '100vh' }}>
            <Sider collapsible collapsed={collapsed} onCollapse={(value) => setCollapsed(value)}>
              <div className="demo-logo-vertical" />
              <Menu theme="dark" selectedKeys={[selectedKey]} mode="inline" items={items} />
            </Sider>
            <Layout>
              <Content style={{ margin: '0 16px' }}>
                {children}
              </Content>
              <Footer style={{ textAlign: 'center' }}>
                Ant Design ©{new Date().getFullYear()} Created by Ant UED
              </Footer>
            </Layout>
          </Layout>
        </AntdRegistry>
      </body>
    </html>
  )
}
