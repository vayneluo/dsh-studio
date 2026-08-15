import { renameSync, writeFileSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

export const DSH_ONBOARDING_VERSION = '0.1.0-rc.6'

const TARGET_PACKAGES = [
  'dsh-client-ui-settings-models',
  'dsh-client-ui-settings-general',
  'dsh-client-ui-model-selection',
]

const scriptsDir = dirname(fileURLToPath(import.meta.url))
const projectRoot = join(scriptsDir, '..')

const readJson = (path) => JSON.parse(readFileSync(path, 'utf8'))
const readText = (path) => readFileSync(path, 'utf8')

const replaceExactlyOnce = (source, anchor, replacement, label) => {
  const first = source.indexOf(anchor)
  const last = source.lastIndexOf(anchor)
  if (first < 0 || first !== last) {
    throw new Error(`${label} anchor must occur exactly once`)
  }
  return `${source.slice(0, first)}${replacement}${source.slice(first + anchor.length)}`
}

const writeAtomically = (path, source) => {
  const temporary = `${path}.provider-onboarding.tmp`
  writeFileSync(temporary, source)
  renameSync(temporary, path)
}

const patchModelsClient = (source) => {
  const overrideDir = join(projectRoot, 'overrides', 'provider-onboarding')
  const css = readText(join(overrideDir, 'provider-wizard.css'))
  const fragments = [
    'provider-catalog.js',
    'provider-wizard.js',
    'provider-wizard-view.js',
  ].map((name) => readText(join(overrideDir, name)).trimEnd())
  const wizard = fragments.join('\n').replace(
    '__PROVIDER_WIZARD_CSS__',
    JSON.stringify(css),
  )
  const dialogRegion = '\t\t//#region lib/types/client/DeepSeekOnboardingDialog.js'
  const injected = replaceExactlyOnce(
    source,
    dialogRegion,
    `${wizard}\n${dialogRegion}`,
    'provider wizard insertion',
  )
  const injectionAnchor = `\t\t\tconst deepSeekOnboardingInjected = () => ({
\t\t\t\tcontroller,
\t\t\t\thooks: { models: controller.store },
\t\t\t\tapi: connection.api,
\t\t\t\tt
\t\t\t});`
  const injectionReplacement = `\t\t\tconst providerSetupController = new ProviderSetupStore(
\t\t\t\tconnection.api,
\t\t\t\tconnection.isLoopback ? "host" : "memory"
\t\t\t);
\t\t\tconst providerSetupInjected = () => ({
\t\t\t\tcontroller,
\t\t\t\tsetupController: providerSetupController,
\t\t\t\thooks: {
\t\t\t\t\tmodels: controller.store,
\t\t\t\t\tproviderSetup: providerSetupController.store
\t\t\t\t},
\t\t\t\tapi: connection.api,
\t\t\t\tt
\t\t\t});`
  const withInjection = replaceExactlyOnce(
    injected,
    injectionAnchor,
    injectionReplacement,
    'provider setup injection',
  )
  const registrationAnchor = `\t\t\tctx.slots.inject("settings.onboarding", () => ctx.slots.register({
\t\t\t\tname: "settings.onboarding",
\t\t\t\tid: "deepseek-official",
\t\t\t\torder: 0,
\t\t\t\tinject: deepSeekOnboardingInjected
\t\t\t}, DeepSeekOnboardingDialog));`
  const registrationReplacement = `\t\t\tctx.slots.inject("settings.onboarding", () => ctx.slots.register({
\t\t\t\tname: "settings.onboarding",
\t\t\t\tid: "provider-setup",
\t\t\t\torder: 0,
\t\t\t\tinject: providerSetupInjected
\t\t\t}, ProviderSetupWizard));`
  let transformed = replaceExactlyOnce(
    withInjection,
    registrationAnchor,
    registrationReplacement,
    'provider setup registration',
  )
  for (const [field, initial] of [
    ['route', 'props.initialRoute ?? ""'],
    ['displayName', 'props.initialDisplayName ?? ""'],
    ['baseURL', 'props.initialBaseURL ?? ""'],
  ]) {
    transformed = replaceExactlyOnce(
      transformed,
      `const [${field}, set${field[0].toUpperCase()}${field.slice(1)}] = (0, react.useState)("");`,
      `const [${field}, set${field[0].toUpperCase()}${field.slice(1)}] = (0, react.useState)(${initial});`,
      `custom provider ${field} initializer`,
    )
  }
  transformed = replaceExactlyOnce(
    transformed,
    'const [protocol, setProtocol] = (0, react.useState)(protocols[0] ?? "");',
    'const [protocol, setProtocol] = (0, react.useState)(props.initialProtocol ?? protocols[0] ?? "");',
    'custom provider protocol initializer',
  )
  transformed = replaceExactlyOnce(
    transformed,
    `\t\t\t\tconst refreshAll = () => {
\t\t\t\t\trefreshModels();
\t\t\t\t\trefreshWelcomeIfLoaded(welcomeController);
\t\t\t\t};`,
    `\t\t\t\tconst refreshAll = () => {
\t\t\t\t\trefreshModels();
\t\t\t\t\trefreshWelcomeIfLoaded(welcomeController);
\t\t\t\t\trefreshProviderSetupIfLoaded(providerSetupController);
\t\t\t\t};`,
    'provider setup connection refresh',
  )
  transformed = replaceExactlyOnce(
    transformed,
    '\t\t\t\t\t\tif (ns === "ui-onboarding") refreshWelcomeIfLoaded(welcomeController);',
    `\t\t\t\t\t\tif (ns === "ui-onboarding") {
\t\t\t\t\t\t\trefreshWelcomeIfLoaded(welcomeController);
\t\t\t\t\t\t\trefreshProviderSetupIfLoaded(providerSetupController);
\t\t\t\t\t\t}`,
    'provider setup settings refresh',
  )
  return transformed
}

const patchGeneralClient = (source) => replaceExactlyOnce(
  source,
  `\t\t\tconst openSection = (0, react.useCallback)((id) => {
\t\t\t\tsetActiveId(id);
\t\t\t\tsetOpen(true);
\t\t\t}, []);`,
  `\t\t\tconst openSection = (0, react.useCallback)((id) => {
\t\t\t\tsetActiveId(id);
\t\t\t\tsetOpen(true);
\t\t\t}, []);
\t\t\t(0, react.useEffect)(() => {
\t\t\t\tconst openModels = () => { openSection("models"); };
\t\t\t\twindow.addEventListener("dsh-studio:open-model-settings", openModels);
\t\t\t\treturn () => { window.removeEventListener("dsh-studio:open-model-settings", openModels); };
\t\t\t}, [openSection]);`,
  'open model settings listener',
)

const patchModelSelectionClient = (source) => {
  let transformed = replaceExactlyOnce(
    source,
    `\t\t\t\t\t\t\t\tstate.status === "ready" && choices.length === 0 && (0, react_jsx_runtime.jsx)("div", {
\t\t\t\t\t\t\t\t\tclassName: ModelSelect_module_css_default.empty,
\t\t\t\t\t\t\t\t\tchildren: t("empty.models")
\t\t\t\t\t\t\t\t})`,
    `\t\t\t\t\t\t\t\tstate.status === "ready" && choices.length === 0 && (0, react_jsx_runtime.jsxs)("div", {
\t\t\t\t\t\t\t\t\tclassName: ModelSelect_module_css_default.empty,
\t\t\t\t\t\t\t\t\tchildren: [
\t\t\t\t\t\t\t\t\t\t(0, react_jsx_runtime.jsx)("span", { children: t("empty.models") }),
\t\t\t\t\t\t\t\t\t\t(0, react_jsx_runtime.jsx)("button", {
\t\t\t\t\t\t\t\t\t\t\ttype: "button",
\t\t\t\t\t\t\t\t\t\t\tclassName: ModelSelect_module_css_default.retry,
\t\t\t\t\t\t\t\t\t\t\tonClick: () => {
\t\t\t\t\t\t\t\t\t\t\t\tclose(true);
\t\t\t\t\t\t\t\t\t\t\t\twindow.dispatchEvent(new CustomEvent("dsh-studio:open-model-settings"));
\t\t\t\t\t\t\t\t\t\t\t},
\t\t\t\t\t\t\t\t\t\t\tchildren: t("action.configureModels")
\t\t\t\t\t\t\t\t\t\t})
\t\t\t\t\t\t\t\t\t]
\t\t\t\t\t\t\t\t})`,
    'empty model settings action',
  )
  transformed = replaceExactlyOnce(
    transformed,
    '\t\t\t"action.reload": "重新加载",',
    '\t\t\t"action.reload": "重新加载",\n\t\t\t"action.configureModels": "配置模型",',
    'configure models zh locale',
  )
  transformed = replaceExactlyOnce(
    transformed,
    '\t\t\t"action.reload": "Reload",',
    '\t\t\t"action.reload": "Reload",\n\t\t\t"action.configureModels": "Configure a model",',
    'configure models en locale',
  )
  return transformed
}

const packageDir = (hostDir, packageName) => join(
  hostDir,
  'node_modules',
  '@deepseek-ai',
  packageName,
)

const assertPackageVersion = (hostDir, packageName) => {
  const manifest = readJson(join(packageDir(hostDir, packageName), 'package.json'))
  if (manifest.version === DSH_ONBOARDING_VERSION) return
  throw new Error(
    `${packageName}: expected ${DSH_ONBOARDING_VERSION}, got ${String(manifest.version)}`,
  )
}

export const applyProviderOnboardingOverride = (hostDir) => {
  for (const packageName of TARGET_PACKAGES) assertPackageVersion(hostDir, packageName)

  const generalHost = join(
    packageDir(hostDir, 'dsh-client-ui-settings-general'),
    'lib',
    'index.js',
  )
  const schemaAnchor = 'const OnboardingSettingsSchema = z.object({ welcomeNoticeVersion: z.string() });'
  const schemaReplacement = 'const OnboardingSettingsSchema = z.object({ welcomeNoticeVersion: z.string(), providerSetupVersion: z.string().required(false) });'
  const generalHostSource = replaceExactlyOnce(
    readText(generalHost),
    schemaAnchor,
    schemaReplacement,
    'onboarding schema',
  )
  const modelsClient = join(
    packageDir(hostDir, 'dsh-client-ui-settings-models'),
    'lib',
    'client.js',
  )
  const modelsClientSource = patchModelsClient(readText(modelsClient))
  const generalClient = join(
    packageDir(hostDir, 'dsh-client-ui-settings-general'),
    'lib',
    'client.js',
  )
  const generalClientSource = patchGeneralClient(readText(generalClient))
  const modelSelectionClient = join(
    packageDir(hostDir, 'dsh-client-ui-model-selection'),
    'lib',
    'client.js',
  )
  const modelSelectionClientSource = patchModelSelectionClient(readText(modelSelectionClient))

  writeAtomically(generalHost, generalHostSource)
  writeAtomically(modelsClient, modelsClientSource)
  writeAtomically(generalClient, generalClientSource)
  writeAtomically(modelSelectionClient, modelSelectionClientSource)
}
