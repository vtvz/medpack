// @deno-types="npm:@types/node"
import { readFileSync, writeFileSync } from "node:fs";
import { PDFDocument, PDFFont, PDFPage, rgb } from "npm:pdf-lib@1.17.1";
import fontkit from "npm:@pdf-lib/fontkit@1.1.1";
import arg from "npm:arg";

const FONT_SIZE = 14;
const TEXT_MARGIN = 4;
const FONT_MARGIN_HORIZONTAL = 10;
let FONT_MARGIN_VERTICAL: number;
let PAGE_MARGIN: number;
let FONT_HEIGHT: number;

async function run() {
  const args = arg({
    "-i": String,
    "-o": String,
    "-r": String,
    "-l": String,
    "-b": String,
    "-f": String,
  });

  const outputPath = args["-o"] as string;
  const inputPath = args["-i"] as string;
  const rightLext = args["-r"] as string;
  const leftLext = args["-l"] as string;
  const bottomText = args["-b"] as string;
  const fontPath = args["-f"] as string;

  const existingPdfBytes = readFileSync(inputPath);
  const pdfDoc = await PDFDocument.load(existingPdfBytes);

  const pages = pdfDoc.getPages();

  pdfDoc.registerFontkit(fontkit);
  const font = await pdfDoc.embedFont(
    readFileSync(fontPath),
  );

  FONT_HEIGHT = font.heightAtSize(FONT_SIZE);
  PAGE_MARGIN = FONT_HEIGHT + TEXT_MARGIN * 2;
  FONT_MARGIN_VERTICAL = FONT_SIZE + TEXT_MARGIN;

  pages.map((page, index) => {
    pages.length;

    extendPdfPages(page);
    drawBox(page);

    addLeftText(
      page,
      font,
      leftLext
        .replace("%Page", `${index + 1}`)
        .replace("%EndPage", `${pages.length}`),
    );
    addRightText(page, font, rightLext);
    addCentreBottom(page, font, bottomText);
  });

  const pdfBytes = await pdfDoc.save();
  writeFileSync(outputPath, pdfBytes);
}

function addLeftText(page: PDFPage, font: PDFFont, text: string) {
  const height = page.getHeight();

  const x = FONT_MARGIN_HORIZONTAL;
  const y = height - FONT_MARGIN_VERTICAL;

  page.drawText(text, {
    x: x,
    y: y,
    size: FONT_SIZE,
    font: font,
    color: rgb(0, 0, 0),
  });
}

function addRightText(page: PDFPage, font: PDFFont, text: string) {
  const { width, height } = page.getSize();

  const textWidth = font.widthOfTextAtSize(text, FONT_SIZE);
  const x = width - textWidth - FONT_MARGIN_HORIZONTAL;
  const y = height - FONT_MARGIN_VERTICAL;

  page.drawText(text, {
    x: x,
    y: y,
    size: FONT_SIZE,
    font: font,
    color: rgb(0, 0, 0),
  });
}

function addCentreBottom(page: PDFPage, font: PDFFont, text: string) {
  const FONT_SIZE = 5;
  const { width } = page.getSize();

  const textWidth = font.widthOfTextAtSize(text, FONT_SIZE);

  const x = (width - textWidth) / 2 - FONT_MARGIN_HORIZONTAL;
  const y = 5;

  page.drawText(text, {
    x: x,
    y: y,
    size: FONT_SIZE,
    font: font,
    color: rgb(0.5, 0.5, 0.5),
  });
}

function extendPdfPages(page: PDFPage) {
  const { width, height } = page.getSize();

  page.setSize(width, height + PAGE_MARGIN);
}

function drawBox(page: PDFPage) {
  const { width, height } = page.getSize();

  page.drawRectangle({
    x: 0,
    y: height - PAGE_MARGIN,
    width: width,
    height: PAGE_MARGIN,
    color: rgb(236 / 256, 236 / 256, 239 / 256),
  });
}

run()
  .then(() => console.log("PDF pages extended successfully!"))
  .catch((err) => console.error("Error extending PDF pages:", err));
