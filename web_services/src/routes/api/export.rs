use actix_web::HttpResponse;
use actix_web::http::header::{self, ContentDisposition, DispositionParam, DispositionType};
use common::api::ApiError;
use rust_xlsxwriter::{Format, Workbook};

pub(crate) const EXPORT_PAGE_SIZE: u32 = 500;

pub(crate) fn xlsx_response(filename: &str, bytes: Vec<u8>) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ))
        .insert_header(ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::Filename(filename.to_string())],
        })
        .body(bytes)
}

pub(crate) fn xlsx_from_rows(
    sheet_name: &str,
    headers: &[&str],
    rows: &[Vec<String>],
) -> Result<Vec<u8>, ApiError> {
    let mut workbook = Workbook::new();
    let header_format = Format::new().set_bold();

    let worksheet = workbook.add_worksheet();
    worksheet.set_name(sheet_name).map_err(|e| {
        tracing::error!("设置 Excel 工作表名称失败: {e}");
        ApiError::Internal("生成 Excel 失败".into())
    })?;

    for (col, header) in headers.iter().enumerate() {
        worksheet
            .write_with_format(0, col as u16, *header, &header_format)
            .map_err(|e| {
                tracing::error!("写入 Excel 表头失败: {e}");
                ApiError::Internal("生成 Excel 失败".into())
            })?;
    }

    for (row_idx, row) in rows.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            worksheet
                .write((row_idx + 1) as u32, col_idx as u16, cell.as_str())
                .map_err(|e| {
                    tracing::error!("写入 Excel 单元格失败: {e}");
                    ApiError::Internal("生成 Excel 失败".into())
                })?;
        }
    }

    worksheet.autofit();
    workbook.save_to_buffer().map_err(|e| {
        tracing::error!("保存 Excel 到内存失败: {e}");
        ApiError::Internal("生成 Excel 失败".into())
    })
}

pub(crate) fn optional<T: ToString>(value: &Option<T>) -> String {
    value.as_ref().map(ToString::to_string).unwrap_or_default()
}

pub(crate) fn datetime(value: &chrono::DateTime<chrono::Utc>) -> String {
    value.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn optional_datetime(value: &Option<chrono::DateTime<chrono::Utc>>) -> String {
    value.as_ref().map(datetime).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{xlsx_from_rows, xlsx_response};
    use actix_web::http::header;

    #[test]
    fn xlsx_generation_returns_zip_package() {
        let bytes = xlsx_from_rows("Sheet", &["名称"], &[vec!["测试".to_string()]]).unwrap();
        assert!(bytes.starts_with(b"PK"));
    }

    #[test]
    fn xlsx_response_uses_excel_content_type() {
        let response = xlsx_response("export.xlsx", vec![1, 2, 3]);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
    }
}
