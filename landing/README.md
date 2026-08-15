# DS Studio 落地页

DS Studio 0.1.3 的纯静态产品落地页。网站与桌面端 `ui/` 相互独立，没有服务端或运行时依赖。

## 本地构建

需要 Node.js 24 或更新版本：

```powershell
Set-Location landing
npm test
npm run build
```

构建完成后，静态文件位于 `landing/dist/`。

## 使用 1Panel 部署

1. 在 1Panel 的“网站”中创建“静态网站”。
2. 将 `landing/dist/` **目录内的全部内容**上传到网站根目录。
3. 默认文档保持为 `index.html`。
4. 绑定域名并申请或导入 HTTPS 证书。

页面不需要反向代理、Node.js 守护进程、数据库或环境变量。

如果使用 Git 仓库拉取构建，请将工作目录设为 `landing`，构建命令设为 `npm run build`，发布目录设为 `dist`。
