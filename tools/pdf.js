const puppeteer = require('puppeteer');

async function print(input, output) {
  let browser = null
  try {
    browser = await puppeteer.launch();
    const page = await browser.newPage();
    await page.goto(`file://${input}`);
    await page.pdf({
      path: output,
      format: 'A4',
      landscape: true,
      printBackground: true
    })
    await browser.close()
  } catch (err) {
    if (browser) {
      await browser.close()
    }
    throw err
  }
}

print(process.argv[2], process.argv[3]).catch(console.error)
