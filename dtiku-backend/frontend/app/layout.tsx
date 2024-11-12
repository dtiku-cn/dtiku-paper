"use client"

import { Layout, Menu, MenuProps } from 'antd'
import './globals.css'
import { useState } from 'react'
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
  getItem(<Link href="/">Dashboard</Link>, '1', <PieChartOutlined />),
  getItem(<Link href="/configs">Config</Link>, '2', <DesktopOutlined />),
  getItem(<Link href="/tasks">Task</Link>, 'sub1', <UserOutlined />),
  getItem('Team', 'sub2', <TeamOutlined />, [getItem('Team 1', '6'), getItem('Team 2', '8')]),
  getItem('Files', '9', <FileOutlined />),
];

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const [collapsed, setCollapsed] = useState(false);
  return (
    <html lang="en">
      <body>
        <AntdRegistry>
          <Layout style={{ minHeight: '100vh' }}>
            <Sider collapsible collapsed={collapsed} onCollapse={(value) => setCollapsed(value)}>
              <div className="demo-logo-vertical" />
              <Menu theme="dark" defaultSelectedKeys={['1']} mode="inline" items={items} />
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
