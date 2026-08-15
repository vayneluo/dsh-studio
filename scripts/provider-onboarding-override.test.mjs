import assert from 'node:assert/strict'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import test from 'node:test'
import { fileURLToPath, pathToFileURL } from 'node:url'

const scriptsDir = dirname(fileURLToPath(import.meta.url))
const projectRoot = join(scriptsDir, '..')
const moduleUrl = pathToFileURL(join(scriptsDir, 'provider-onboarding-override.mjs')).href
const packageNames = [
  'dsh-client-ui-settings-models',
  'dsh-client-ui-settings-general',
  'dsh-client-ui-model-selection',
]

const modelsClientFixture = `function modelsClientFixture() {
\t\t//#region lib/types/client/DeepSeekOnboardingDialog.js
\t\tfunction CustomProviderCard(props) {
\t\t\tconst protocols = [];
\t\t\tconst [route, setRoute] = (0, react.useState)("");
\t\t\tconst [displayName, setDisplayName] = (0, react.useState)("");
\t\t\tconst [baseURL, setBaseURL] = (0, react.useState)("");
\t\t\tconst [protocol, setProtocol] = (0, react.useState)(protocols[0] ?? "");
\t\t}
\t\tfunction install(ctx, controller, welcomeController, connection, t) {
\t\t\tconst deepSeekOnboardingInjected = () => ({
\t\t\t\tcontroller,
\t\t\t\thooks: { models: controller.store },
\t\t\t\tapi: connection.api,
\t\t\t\tt
\t\t\t});
\t\t\tctx.effect(() => {
\t\t\t\tconst refreshModels = () => {
\t\t\t\t\trefreshIfLoaded(controller);
\t\t\t\t};
\t\t\t\tconst refreshAll = () => {
\t\t\t\t\trefreshModels();
\t\t\t\t\trefreshWelcomeIfLoaded(welcomeController);
\t\t\t\t};
\t\t\t\tconst disposers = [
\t\t\t\t\tctx.remote.$on("settings/document-updated", (ns) => {
\t\t\t\t\t\trefreshModels();
\t\t\t\t\t\tif (ns === "ui-onboarding") refreshWelcomeIfLoaded(welcomeController);
\t\t\t\t\t}),
\t\t\t\t\tctx.on("connection/reset", refreshAll)
\t\t\t\t];
\t\t\t\treturn () => { for (const dispose of disposers) dispose(); };
\t\t\t});
\t\t\tctx.slots.inject("settings.onboarding", () => ctx.slots.register({
\t\t\t\tname: "settings.onboarding",
\t\t\t\tid: "deepseek-official",
\t\t\t\torder: 0,
\t\t\t\tinject: deepSeekOnboardingInjected
\t\t\t}, DeepSeekOnboardingDialog));
\t\t}
}
`

const generalHostFixture = 'const OnboardingSettingsSchema = z.object({ welcomeNoticeVersion: z.string() });\n'

const generalClientFixture = `function SettingsRoot() {
\t\t\tconst openSection = (0, react.useCallback)((id) => {
\t\t\t\tsetActiveId(id);
\t\t\t\tsetOpen(true);
\t\t\t}, []);
}
`

const modelSelectionClientFixture = `function ModelSelect() {
\tconst state = { status: "ready" };
\tconst choices = [];
\tconst close = () => {};
\tconst t = (key) => key;
\treturn [
\t\t\t\t\t\t\t\tstate.status === "ready" && choices.length === 0 && (0, react_jsx_runtime.jsx)("div", {
\t\t\t\t\t\t\t\t\tclassName: ModelSelect_module_css_default.empty,
\t\t\t\t\t\t\t\t\tchildren: t("empty.models")
\t\t\t\t\t\t\t\t})
\t];
}
const zh = {
\t\t\t"action.reload": "重新加载",
\t\t\t"empty.models": "没有可用的模型。"
};
const en = {
\t\t\t"action.reload": "Reload",
\t\t\t"empty.models": "No models available."
};
`

const makeRuntimeFixture = (t) => {
  const root = mkdtempSync(join(tmpdir(), 'dsh-provider-onboarding-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const scope = join(root, 'node_modules', '@deepseek-ai')
  for (const packageName of packageNames) {
    const target = join(scope, packageName)
    mkdirSync(join(target, 'lib'), { recursive: true })
    writeFileSync(join(target, 'package.json'), JSON.stringify({
      name: `@deepseek-ai/${packageName}`,
      version: '0.1.0-rc.6',
    }))
  }
  writeFileSync(modelsClientPath(root), modelsClientFixture)
  writeFileSync(generalHostPath(root), generalHostFixture)
  writeFileSync(generalClientPath(root), generalClientFixture)
  writeFileSync(modelSelectionClientPath(root), modelSelectionClientFixture)
  return root
}

const manifestPath = (host, packageName) => join(
  host,
  'node_modules',
  '@deepseek-ai',
  packageName,
  'package.json',
)

const generalHostPath = (host) => join(
  host,
  'node_modules',
  '@deepseek-ai',
  'dsh-client-ui-settings-general',
  'lib',
  'index.js',
)

const modelsClientPath = (host) => join(
  host,
  'node_modules',
  '@deepseek-ai',
  'dsh-client-ui-settings-models',
  'lib',
  'client.js',
)

const generalClientPath = (host) => join(
  host,
  'node_modules',
  '@deepseek-ai',
  'dsh-client-ui-settings-general',
  'lib',
  'client.js',
)

const modelSelectionClientPath = (host) => join(
  host,
  'node_modules',
  '@deepseek-ai',
  'dsh-client-ui-model-selection',
  'lib',
  'client.js',
)

test('exports a provider onboarding runtime override', async () => {
  let subject
  try {
    subject = await import(moduleUrl)
  } catch {
    subject = undefined
  }

  assert.equal(typeof subject?.applyProviderOnboardingOverride, 'function')
})

test('rejects an unsupported Harness package version before editing files', async (t) => {
  const host = makeRuntimeFixture(t)
  const path = manifestPath(host, 'dsh-client-ui-settings-models')
  const manifest = JSON.parse(readFileSync(path, 'utf8'))
  writeFileSync(path, JSON.stringify({ ...manifest, version: '0.1.0-rc.7' }))
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  assert.throws(
    () => applyProviderOnboardingOverride(host),
    /dsh-client-ui-settings-models.*expected 0\.1\.0-rc\.6.*0\.1\.0-rc\.7/i,
  )
})

test('extends the durable onboarding schema with Schemastery optional-field syntax', async (t) => {
  const host = makeRuntimeFixture(t)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  applyProviderOnboardingOverride(host)

  const source = readFileSync(generalHostPath(host), 'utf8')
  assert.match(source, /providerSetupVersion: z\.string\(\)\.required\(false\)/)
})

test('rejects a missing transform anchor without changing the target', async (t) => {
  const host = makeRuntimeFixture(t)
  const path = generalHostPath(host)
  const broken = readFileSync(path, 'utf8').replace(
    'const OnboardingSettingsSchema = z.object({ welcomeNoticeVersion: z.string() });',
    'const OnboardingSettingsSchema = z.object({});',
  )
  writeFileSync(path, broken)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  assert.throws(() => applyProviderOnboardingOverride(host), /onboarding schema anchor.*exactly once/i)
  assert.equal(readFileSync(path, 'utf8'), broken)
})

test('rejects applying the same override twice', async (t) => {
  const host = makeRuntimeFixture(t)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  applyProviderOnboardingOverride(host)

  assert.throws(() => applyProviderOnboardingOverride(host), /onboarding schema anchor.*exactly once/i)
})

test('replaces the DeepSeek-only registration with durable provider setup', async (t) => {
  const host = makeRuntimeFixture(t)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  applyProviderOnboardingOverride(host)

  const source = readFileSync(modelsClientPath(host), 'utf8')
  assert.match(source, /const PROVIDER_SETUP_VERSION = "1"/)
  assert.match(source, /class ProviderSetupStore/)
  assert.match(source, /id: "provider-setup"/)
  assert.match(source, /path: \[PROVIDER_SETUP_FIELD\]/)
  assert.doesNotMatch(
    source,
    /id: "deepseek-official",\s+order: 0,[\s\S]{0,160}DeepSeekOnboardingDialog/,
  )
})

test('injects the approved two-step provider chooser and reuses Models editors', async (t) => {
  const host = makeRuntimeFixture(t)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  applyProviderOnboardingOverride(host)

  const source = readFileSync(modelsClientPath(host), 'utf8')
  for (const expected of [
    'DeepSeek',
    '阿里云百炼',
    '火山方舟',
    '智谱 AI',
    'OpenAI',
    'Anthropic',
    'Gemini',
    'OpenRouter',
    'Ollama',
    'OpenAI Compatible',
  ]) assert.match(source, new RegExp(expected))
  assert.match(source, /function ProviderCard/)
  assert.match(source, /ProviderEditor/)
  assert.match(source, /CustomProviderCard/)
  assert.match(source, /provider-wizard__progress/)
  assert.match(source, /provider-wizard__grid/)
  assert.doesNotMatch(source, /https:\/\/cdn\.|<script[^>]+src=/)
})

test('keeps provider selection separate from configuration and prefills compatible presets', async (t) => {
  const host = makeRuntimeFixture(t)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  applyProviderOnboardingOverride(host)

  const source = readFileSync(modelsClientPath(host), 'utf8')
  assert.match(source, /const \[step, setStep\] = \(0, react\.useState\)\(1\)/)
  assert.match(source, /onClick: \(\) => \{ setStep\(2\); \}/)
  assert.match(source, /react\.useState\)\(props\.initialRoute \?\? ""\)/)
  assert.match(source, /react\.useState\)\(props\.initialBaseURL \?\? ""\)/)
  assert.match(source, /react\.useState\)\(props\.initialProtocol \?\? protocols\[0\] \?\? ""\)/)
})

test('keeps the selected provider editor mounted when navigating back', async (t) => {
  const host = makeRuntimeFixture(t)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  applyProviderOnboardingOverride(host)

  const source = readFileSync(modelsClientPath(host), 'utf8')
  assert.match(source, /const showingSelection = step === 1 \|\| selected === void 0/)
  assert.match(source, /className: "provider-wizard__selection",\s+hidden: !showingSelection/)
  assert.match(source, /className: "provider-wizard__configuration",\s+hidden: showingSelection/)
})

test('refreshes durable provider setup state on settings and connection changes', async (t) => {
  const host = makeRuntimeFixture(t)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  applyProviderOnboardingOverride(host)

  const source = readFileSync(modelsClientPath(host), 'utf8')
  assert.match(source, /function refreshProviderSetupIfLoaded\(controller\)/)
  assert.match(source, /refreshProviderSetupIfLoaded\(providerSetupController\)/)
  const invalidation = source.match(/ctx\.remote\.\$on\("settings\/document-updated"[\s\S]{0,420}/)?.[0] ?? ''
  assert.match(invalidation, /refreshProviderSetupIfLoaded/)
})

test('opens Models settings from the empty model selector with one local event', async (t) => {
  const host = makeRuntimeFixture(t)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  applyProviderOnboardingOverride(host)

  const selection = readFileSync(modelSelectionClientPath(host), 'utf8')
  assert.match(selection, /"action\.configureModels": "配置模型"/)
  assert.match(selection, /"action\.configureModels": "Configure a model"/)
  assert.match(selection, /children: t\("empty\.models"\)/)
  assert.match(selection, /children: t\("action\.configureModels"\)/)
  assert.equal(selection.match(/dispatchEvent\(new CustomEvent\("dsh-studio:open-model-settings"\)\)/g)?.length, 1)

  const general = readFileSync(generalClientPath(host), 'utf8')
  assert.equal(general.match(/addEventListener\("dsh-studio:open-model-settings", openModels\)/g)?.length, 1)
  assert.equal(general.match(/removeEventListener\("dsh-studio:open-model-settings", openModels\)/g)?.length, 1)
  assert.match(general, /const openModels = \(\) => \{ openSection\("models"\); \};/)
  assert.doesNotMatch(`${selection}\n${general}`, /window\.openModelSettings\s*=/)
})

test('matches native provider rows with the compatible preset aliases', async (t) => {
  const host = makeRuntimeFixture(t)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  applyProviderOnboardingOverride(host)

  const source = readFileSync(modelsClientPath(host), 'utf8')
  assert.match(source, /aliases: \["alibaba", "qwen", "bailian"\]/)
  assert.match(source, /aliases: \["volcengine", "doubao", "volcengine-ark"\]/)
  assert.match(source, /id: "ollama"[\s\S]{0,420}baseURL: "http:\/\/127\.0\.0\.1:11434\/v1"/)
})

test('documents local monograms and provider trademark ownership', () => {
  const path = join(projectRoot, 'overrides', 'provider-onboarding', 'THIRD_PARTY_NOTICES.md')
  assert.equal(existsSync(path), true)
  const notice = readFileSync(path, 'utf8')
  assert.match(notice, /monogram/i)
  assert.match(notice, /trademark/i)
  assert.match(notice, /no remote/i)
})

test('produces syntactically valid transformed client bundles', async (t) => {
  const host = makeRuntimeFixture(t)
  const { applyProviderOnboardingOverride } = await import(moduleUrl)

  applyProviderOnboardingOverride(host)

  for (const path of [
    modelsClientPath(host),
    generalClientPath(host),
    modelSelectionClientPath(host),
  ]) execFileSync(process.execPath, ['--check', path], { stdio: 'pipe' })
})
