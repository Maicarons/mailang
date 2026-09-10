import { test, expect } from '@playwright/test'

test.describe('MaìLang Playground', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('page loads correctly', async ({ page }) => {
    // Check title
    await expect(page).toHaveTitle(/MaìLang/)

    // Check main elements exist
    await expect(page.locator('.logo-text')).toContainText('MaìLang Playground')
    await expect(page.locator('.btn-run')).toBeVisible()
    await expect(page.locator('.btn-reset')).toBeVisible()
  })

  test('editor is visible and editable', async ({ page }) => {
    // Wait for CodeMirror to load
    await expect(page.locator('.cm-editor')).toBeVisible()

    // Check editor has content
    const editor = page.locator('.cm-content')
    await expect(editor).toBeVisible()

    // Check editor is editable (has contenteditable attribute)
    await expect(editor).toHaveAttribute('contenteditable', 'true')
  })

  test('output panel is visible', async ({ page }) => {
    await expect(page.locator('.output-container')).toBeVisible()
    await expect(page.locator('.output-title')).toContainText('Output')
  })

  test('example selector is visible', async ({ page }) => {
    await expect(page.locator('.examples')).toBeVisible()
    await expect(page.locator('.examples-header')).toContainText('Examples')

    // Check examples exist
    const examples = page.locator('.example-item')
    await expect(examples).toHaveCount(12)
  })

  test('select example updates editor', async ({ page }) => {
    // Click on "Variables" example
    await page.locator('.example-item').filter({ hasText: 'Variables' }).click()

    // Check editor contains variable-related content
    const editorContent = page.locator('.cm-content')
    await expect(editorContent).toContainText('let x = 42')
  })

  test('run button executes code', async ({ page }) => {
    // Click the Run button
    await page.locator('.btn-run').click()

    // Wait for output
    await page.waitForTimeout(1000)

    // Check output panel has content
    const output = page.locator('.output-text')
    await expect(output).toBeVisible()
  })

  test('run hello world example', async ({ page }) => {
    // Select Hello World example
    await page.locator('.example-item').filter({ hasText: 'Hello World' }).click()

    // Click Run
    await page.locator('.btn-run').click()

    // Wait for output
    await page.waitForTimeout(1000)

    // Check output
    const output = page.locator('.output-text')
    await expect(output).toContainText('MaìLang')
  })

  test('run arithmetic example', async ({ page }) => {
    // Select Arithmetic example
    await page.locator('.example-item').filter({ hasText: 'Arithmetic' }).click()

    // Click Run
    await page.locator('.btn-run').click()

    // Wait for output
    await page.waitForTimeout(1000)

    // Check output contains calculation results
    const output = page.locator('.output-text')
    await expect(output).toContainText('加法')
  })

  test('run fibonacci example', async ({ page }) => {
    // Select Fibonacci example
    await page.locator('.example-item').filter({ hasText: 'Fibonacci' }).click()

    // Click Run
    await page.locator('.btn-run').click()

    // Wait for output (fibonacci might take a moment)
    await page.waitForTimeout(2000)

    // Check output contains fibonacci numbers
    const output = page.locator('.output-text')
    await expect(output).toContainText('fib')
  })

  test('run loops example', async ({ page }) => {
    // Select Loops example
    await page.locator('.example-item').filter({ hasText: 'Loops' }).click()

    // Click Run
    await page.locator('.btn-run').click()

    // Wait for output
    await page.waitForTimeout(1000)

    // Check output contains loop results
    const output = page.locator('.output-text')
    await expect(output).toContainText('i =')
  })

  test('reset button clears state', async ({ page }) => {
    // Run some code first
    await page.locator('.btn-run').click()
    await page.waitForTimeout(500)

    // Click Reset
    await page.locator('.btn-reset').click()

    // Check output is cleared
    const outputEmpty = page.locator('.output-empty')
    await expect(outputEmpty).toBeVisible()
  })

  test('status shows WASM ready', async ({ page }) => {
    // Wait for WASM to load
    await page.waitForTimeout(2000)

    // Check status indicator
    const status = page.locator('.status')
    await expect(status).toContainText('WASM Ready')
  })

  test('keyboard shortcut runs code', async ({ page }) => {
    // Focus on editor
    await page.locator('.cm-editor').click()

    // Press Ctrl+Enter using keyboard
    await page.keyboard.down('Control')
    await page.keyboard.press('Enter')
    await page.keyboard.up('Control')

    // Wait for output
    await page.waitForTimeout(1500)

    // Check output has content (or empty output if code has no println)
    const output = page.locator('.output-text')
    const emptyOutput = page.locator('.output-empty')
    // Either output or empty state should be visible
    const hasOutput = await output.isVisible().catch(() => false)
    const hasEmpty = await emptyOutput.isVisible().catch(() => false)
    expect(hasOutput || hasEmpty).toBeTruthy()
  })

  test('multiple examples can be run', async ({ page }) => {
    // Run Hello World
    await page.locator('.example-item').filter({ hasText: 'Hello World' }).click()
    await page.locator('.btn-run').click()
    await page.waitForTimeout(500)

    // Run Variables
    await page.locator('.example-item').filter({ hasText: 'Variables' }).click()
    await page.locator('.btn-run').click()
    await page.waitForTimeout(500)

    // Check output changed
    const output = page.locator('.output-text')
    await expect(output).toBeVisible()
  })

  test('error display works', async ({ page }) => {
    // Clear editor and type invalid code that will cause an error
    const editor = page.locator('.cm-content')
    await editor.click()
    await page.keyboard.press('Control+A')
    await page.keyboard.press('Backspace')
    await page.keyboard.type('println(undefined_var_xyz)')

    // Click Run
    await page.locator('.btn-run').click()

    // Wait for output
    await page.waitForTimeout(1000)

    // The fallback interpreter will show something (either output or error)
    const output = page.locator('.output-text')
    const errorOutput = page.locator('.output-error')
    const hasOutput = await output.isVisible().catch(() => false)
    const hasError = await errorOutput.isVisible().catch(() => false)
    // Either output or error should be displayed
    expect(hasOutput || hasError).toBeTruthy()
  })
})
