# MedPack

[![Rust](https://img.shields.io/badge/rust-nightly--2026--09--11-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> **📢 Subscribe to the author's telegram channel for updates and more projects:** [**@vtvz_dev**](https://t.me/vtvz_dev)

A powerful Rust-based tool for processing and organizing medical documents from Telegram chat exports into structured PDF documents with automatic OCR, metadata extraction, and comprehensive table of contents generation.

## 🎯 Overview

MedPack transforms structured Telegram chat exports containing medical records into beautifully organized PDF documents. It intelligently processes images, PDFs, and text messages, groups them by person, and creates professional medical document collections with proper pagination, OCR processing, and detailed table of contents.

### Key Features

- **📱 Multi-format Processing**: Handles images (PNG, JPG), PDFs, and text messages from Telegram exports
- **🔍 OCR Integration**: Automatic OCR processing for images using `ocrmypdf` with Russian and English language support
- **📋 Metadata Extraction**: Parses YAML metadata blocks from messages to extract structured medical record information
- **👥 Smart Organization**: Groups messages by person and creates separate PDF documents for each individual
- **📚 Table of Contents**: Generates detailed TOC with page numbers, dates, tags, and clickable Telegram message links
- **⚡ Parallel Processing**: Multi-threaded processing with real-time progress bars for efficient handling of large datasets
- **🏷️ Document Labeling**: Adds professional headers, footers, and page numbers to all documents
- **🛠️ Flexible Configuration**: Optional OCR processing, temporary file preservation for debugging
- **🔗 Telegram Integration**: Preserves links to original messages for easy reference

## 📺 Live Example

Want to see MedPack in action? Check out our **live example Telegram group**:

**🔗 [MedPack Example Group](https://t.me/medpack_example)**

This group contains:

- **📱 Real medical record messages** with proper YAML metadata formatting
- **🖼️ Sample images and PDFs** showing the input format MedPack expects
- **📄 Processing results** - the final generated PDF documents
- **💡 Best practices** for structuring your medical records in Telegram

The group demonstrates exactly how to format your Telegram messages for optimal MedPack processing, including proper YAML metadata blocks, image attachments, and text formatting. You can use this as a reference when preparing your own medical record exports.

## 🚀 Quick Start

The easiest way to run MedPack is using **Docker**. All prerequisites are preinstalled in the image.

```bash
docker run --rm -v "$(pwd):$(pwd)" -w "$(pwd)" -u "$(id -u):$(id -g)" -it --pull always ghcr.io/vtvz/medpack:latest
```

### Building from Source

#### Prerequisites

Before using MedPack, ensure you have all the required external tools installed. The complete list of required tools can be found in the `src/command.rs` file.

#### Building MedPack

1. **Clone the repository:**

```bash
git clone <repository-url>
cd medpack
```

2. **Build the project:**

```bash
cargo build --release
```

The binary will be available at `target/release/medpack`.

#### Installing MedPack

Alternatively, you can install MedPack directly to your system using Cargo:

```bash
cargo install --path .
```

This will install the `medpack` binary to your Cargo bin directory (usually `~/.cargo/bin/`), making it available system-wide.

## 📖 Usage

### Basic Usage

```bash
medpack [OPTIONS] [SOURCES...]
```

### Command Line Options

For a complete list of available options and their descriptions, run:

```bash
medpack --help
```

### Examples

**Process current directory:**

```bash
medpack
```

**Process specific directories without OCR:**

```bash
medpack --no-ocr /path/to/export1 /path/to/export2
```

**Debug mode with temporary file preservation:**

```bash
medpack --preserve-tmp --no-ocr ./telegram_export
```

**Process multiple exports simultaneously:**

```bash
medpack ~/Downloads/ChatExport_2023 ~/Downloads/ChatExport_2024
```

> **💡 Tip**: When processing multiple exports, MedPack will merge them together. This allows you to process only new days in the future instead of re-exporting the entire chat history - simply export the new messages and process them alongside your existing exports.
>
> **📝 Note**: When merging exports that contain the same messages (including edited versions), MedPack automatically uses the latest edited version of each message. This ensures that any corrections or updates made to medical records in Telegram are properly reflected in the final PDF output.

## 📁 Input Format

### Telegram Export Structure

MedPack expects Telegram chat exports in JSON format with the following structure:

```
telegram_export/
├── result.json          # Main export file with message data
├── photos/             # Directory containing image files
│   ├── photo_1.jpg
│   └── photo_2.png
└── files/              # Directory containing PDF attachments
    └── document.pdf
```

### Message Types Processed

⚠️ **Important**: Only messages containing YAML metadata blocks are processed. All other messages, images, and files without YAML blocks are ignored.

1. **📝 Messages with YAML metadata blocks** - Define medical records with structured information
2. **📷 Image messages** - Photos in PNG or JPEG format (both compressed regular photos and uncompressed file attachments) that can be processed with OCR
3. **📄 PDF attachments** - Direct PDF files from messages
4. **💬 Text messages** - Converted to PDF format

### YAML Metadata Format

Messages MUST contain YAML blocks with medical record metadata:

```yaml
date: 2023.12.22
person: John Doe
tags:
  - cardiology
  - checkup
  - ECG
place: City Hospital
doctor: Dr. Smith
```

#### 📝 Text Record Formatting

For text-only records (messages without images or PDF files), you can use special code blocks to enhance the content:

**HTML Code Blocks** - Insert raw HTML directly into the generated PDF

**CSV Code Blocks** - Create tables from CSV data, where the first row is treated as the header

**Hidden Code Blocks** - Add personal notes that won't appear in the final PDF

**Telegram Formatting** - All Telegram message formatting is preserved

#### 📑 Rich Text Messages

Newer Telegram exports can contain **rich text messages** (`rich_message` in `result.json`) instead of plain `text_entities`. MedPack fully supports them, rendering all rich blocks into the PDF:

- **Tables** - Native Telegram tables with headers, alignment, borders, and striping
- **Headings** - Levels 1–6
- **Quotes** - Regular and pull quotes with captions
- **Lists** - Ordered, bullet, and nested lists, including checklists (☐/☑)
- **Details** - Collapsible sections are rendered as a bold title with always-visible content
- **Code blocks** - The same `html`, `csv`, and `hidden` languages work as in regular messages
- **Inline formatting** - Bold, italic, underline, strikethrough, spoiler, subscript, superscript, highlight, and links
- **Dividers, footers, and URL buttons**

The YAML metadata block must still be the **first block** of the rich message (a `yaml` code block).

<details>
<summary><strong>Example Text Record:</strong></summary>

````
```yaml
date: 2023.12.22
person: John Doe
tags:
  - consultation
  - notes
```

Patient reported feeling better after treatment.

```html
<div class="alert alert-info">
  <strong>Important:</strong> Patient has allergies to penicillin and sulfa drugs.
</div>
```

```csv
Medication,Dosage,Frequency,Duration
Aspirin,100mg,Daily,30 days
Lisinopril,10mg,Daily,Ongoing
Metformin,500mg,Twice daily,90 days
```

Follow-up appointment scheduled for next month.

```hidden
Remember to follow up on blood test results next week.
Patient seemed anxious - consider referral to counselor.
```
````

</details>

#### ⚠️ Important YAML Block Requirements

- **📍 Position**: The YAML block **must be at the very beginning** of the message text
- **📷 Multiple Images**: If a medical record consists of multiple images, the YAML block should be placed **under the first image** in the sequence
- **🖼️ Image Format**: Images must be in **PNG or JPEG format** (both compressed regular photos and uncompressed file attachments) for proper OCR processing
- **💻 Formatting**: The YAML block must be formatted as code within the Telegram message, not as plain text

#### Supported YAML Fields

| Field    | Type   | Description                              | Required |
| -------- | ------ | ---------------------------------------- | -------- |
| `date`   | String | Date of the medical record (YYYY.MM.DD)  | ✅       |
| `person` | String | Name of the person the record belongs to | ✅       |
| `tags`   | Array  | List of tags/categories for the record   | ✅       |
| `place`  | String | Medical facility or location             | ❌       |
| `doctor` | String | Doctor's name                            | ❌       |

#### 🏷️ HTML Tags Support

Tags now support HTML formatting for enhanced visual presentation in the generated PDFs. This is particularly useful for highlighting important issues or categorizing records with visual emphasis.

**Examples:**

```yaml
tags:
  - cardiology
  - <b>urgent</b>
  - <i>follow-up required</i>
  - ECG
  - <b style="color: red;">critical</b>
```

## 📤 Output

### Generated Files

For each person found in the chat export, MedPack generates:

1. **`PersonName.pdf`** - Complete medical document collection
2. **Table of Contents** - At the beginning of each PDF containing:
   - Record dates and tags
   - Page numbers with proper pagination
   - Clickable links to original Telegram messages
   - Doctor and location information
   - Professional formatting with Bootstrap CSS

### Document Features

- **📄 Professional Layout**: Clean, medical-grade document formatting
- **🔢 Page Numbers**: Consistent pagination throughout the document
- **🏷️ Headers & Footers**: Record metadata displayed in document headers
- **🔗 Telegram Links**: Direct links to original messages for verification
- **📊 Progress Tracking**: Real-time progress bars during processing
- **🎨 Responsive Design**: Bootstrap-based HTML rendering for PDFs

## 🐛 Troubleshooting

### Common Issues

#### Missing External Tools

```bash
# Error: command not found
medpack: error: `img2pdf` not found in PATH
```

**Solution**: Install missing prerequisites using your package manager.

#### OCR Processing Slow

```bash
# Use --no-ocr flag to completely disable OCR processing
medpack --no-ocr
```

**Note**: The `--no-ocr` flag completely disables OCR processing for images, which significantly speeds up processing but means that text within images will not be extracted or searchable in the final PDF.

### Debug Mode

Enable debug mode to inspect temporary files:

```bash
medpack --preserve-tmp
```

This will output paths to temporary directories:

```
tmp folders: /tmp/medpack_html_xyz /tmp/medpack_img_xyz /tmp/medpack_label_xyz
```

## 📄 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

**📧 Support**: For issues and questions, please use the GitHub issue tracker.

**🔄 Updates**: Check releases for the latest features and bug fixes.

**💬 Personal Support**: If you have any questions or need help, feel free to reach out to me personally on Telegram: [@vtvz_me](https://t.me/vtvz_me)
