// @deno-types="npm:@types/node"
import { readFileSync, writeFileSync } from "node:fs";
import { PDFDocument, PDFFont, PDFPage, rgb } from "npm:pdf-lib@1.17.1";
import fontkit from "npm:@pdf-lib/fontkit@1.1.1";
import arg from "npm:arg";
import process from "node:process";

class Config {
  outputPath: string = "";
  inputPath: string = "";
  rightLext: string = "";
  leftLext: string = "";
  bottomText: string = "";
  fontPath: string = "";

  static fromArgs(argv: string[]): Config {
    const config = new Config();
    const args = arg({
      "-i": String,
      "-o": String,
      "-r": String,
      "-l": String,
      "-b": String,
      "-f": String,
    }, { argv });

    config.outputPath = args["-o"] as string;
    config.inputPath = args["-i"] as string;
    config.rightLext = args["-r"] as string;
    config.leftLext = args["-l"] as string;
    config.bottomText = args["-b"] as string;
    config.fontPath = args["-f"] as string;

    return config;
  }
}

class App {
  fontSize = 14;
  textMarginVertical = 4;
  textMarginHorizontal = 10;

  textTopShift = 0;
  pageTopExtend = 0;
  fontHeight = 0;

  config: Config;

  constructor(config: Config) {
    this.config = config;
  }

  prepare(font: PDFFont) {
    this.fontHeight = font.heightAtSize(this.fontSize);
    this.pageTopExtend = this.fontHeight + this.textMarginVertical * 2;
    this.textTopShift = this.fontSize + this.textMarginVertical;
  }

  async run() {
    const existingPdfBytes = readFileSync(this.config.inputPath);
    const pdfDoc = await PDFDocument.load(existingPdfBytes);

    pdfDoc.registerFontkit(fontkit);
    const font = await pdfDoc.embedFont(
      readFileSync(this.config.fontPath),
    );

    this.prepare(font);

    const pages = pdfDoc.getPages();

    pages.map((page, index) => {
      pages.length;

      this.extendPdfPages(page);
      this.drawBox(page);

      this.addLeftText(
        page,
        font,
        this.config.leftLext
          .replace("%Page", `${index + 1}`)
          .replace("%EndPage", `${pages.length}`),
      );
      this.addRightText(page, font, this.config.rightLext);
      this.addCentreBottom(page, font, this.config.bottomText);
    });

    const pdfBytes = await pdfDoc.save();
    writeFileSync(this.config.outputPath, pdfBytes);
  }

  addLeftText(page: PDFPage, font: PDFFont, text: string) {
    const height = page.getHeight();

    const x = this.textMarginHorizontal;
    const y = height - this.textTopShift;

    page.drawText(text, {
      x: x,
      y: y,
      size: this.fontSize,
      font: font,
      color: rgb(0, 0, 0),
    });
  }

  addRightText(page: PDFPage, font: PDFFont, text: string) {
    const { width, height } = page.getSize();

    const textWidth = font.widthOfTextAtSize(text, this.fontSize);
    const x = width - textWidth - this.textMarginHorizontal;
    const y = height - this.textTopShift;

    page.drawText(text, {
      x: x,
      y: y,
      size: this.fontSize,
      font: font,
      color: rgb(0, 0, 0),
    });
  }

  addCentreBottom(page: PDFPage, font: PDFFont, text: string) {
    const FONT_SIZE = 5;
    const { width } = page.getSize();

    const textWidth = font.widthOfTextAtSize(text, FONT_SIZE);

    const x = (width - textWidth) / 2 - this.textMarginHorizontal;
    const y = 5;

    page.drawText(text, {
      x: x,
      y: y,
      size: FONT_SIZE,
      font: font,
      color: rgb(0.5, 0.5, 0.5),
    });
  }

  extendPdfPages(page: PDFPage) {
    const { width, height } = page.getSize();

    page.setSize(width, height + this.pageTopExtend);
  }

  drawBox(page: PDFPage) {
    const { width, height } = page.getSize();

    page.drawRectangle({
      x: 0,
      y: height - this.pageTopExtend,
      width: width,
      height: this.pageTopExtend,
      color: rgb(236 / 256, 236 / 256, 239 / 256),
    });
  }
}

(new App(Config.fromArgs(process.argv))).run()
  .then(() => console.log("PDF pages extended successfully!"))
  .catch((err) => console.error("Error extending PDF pages:", err));
