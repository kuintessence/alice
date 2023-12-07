use actix_http::body::{BoxBody, EitherBody};
use actix_http::{BoxedPayloadStream, Payload, StatusCode};
use actix_i18n::Locale as FluentLocale;
use actix_web::dev::ServiceResponse;
use actix_web::http::header::ContentType;
use actix_web::middleware::ErrorHandlerResponse;
use actix_web::{dev, HttpResponse, HttpResponseBuilder, Result};
use actix_web::{FromRequest, HttpRequest};
use alice_architecture::response::{Locale, LocalizedMsg};

use crate::error::{AliceCommonError, AliceError};

pub struct AliceFluentLocale {
    locale: FluentLocale,
}

impl Locale for AliceFluentLocale {
    fn text_with_args(
        &self,
        id: &str,
        args: std::collections::HashMap<String, String>,
    ) -> anyhow::Result<String> {
        Ok(self.locale.text_with_args(id, args)?)
    }

    fn text(&self, id: &str) -> anyhow::Result<String> {
        Ok(self.locale.text(id)?)
    }
}

pub fn add_error_header<B>(res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>>
where
    B: 'static,
{
    let x = Box::pin(async {
        let req = res.request();
        let mut payload = Payload::<BoxedPayloadStream>::None;
        let locale = FluentLocale::from_request(req, &mut payload).await.unwrap();
        let fluent_locale = Box::new(AliceFluentLocale { locale });

        let mut www_authen_response_builder = HttpResponseBuilder::new(StatusCode::default());

        let (new_req, res) = res.into_parts();

        let error = match res.error() {
            Some(e) => e,
            None => return Ok(ServiceResponse::new(new_req, res).map_into_left_body()),
        };

        let invalid_req = AliceError::new(AliceCommonError::InvalidRequest {
            error_description: error.to_string(),
        });
        let co_error = error.as_error::<AliceError>().or_else(|| {
            if res.status() == 400 {
                Some(&invalid_req)
            } else {
                None
            }
        });

        if co_error.is_none() {
            return Ok(localized_msg_to_service_response(
                new_req,
                fallback_response(&fluent_locale),
            ));
        }

        let co_error = co_error.unwrap();
        let status = co_error.0.status();
        let req_path = new_req.path().to_owned();

        // 400、401、403 don't need to be localized.
        match status {
            400 | 401 | 403 => {
                let response = www_authen_response_builder
                    .status(match status {
                        400 => StatusCode::BAD_REQUEST,
                        401 => StatusCode::UNAUTHORIZED,
                        403 => StatusCode::FORBIDDEN,
                        _ => unreachable!(),
                    })
                    .insert_header((actix_http::header::WWW_AUTHENTICATE, co_error.to_string()))
                    .finish();
                tracing::error!("Handling {req_path}: {status}");
                return Ok(ServiceResponse::new(new_req, response).map_into_right_body());
            }
            _ => {}
        };

        let mut exception_status = 0;
        let new_res = if let Some(e) = res.error() {
            let co_e = e.as_error::<AliceError>();
            let body = BoxBody::new(
                serde_json::to_string(&match co_e {
                    Some(e) => {
                        exception_status = e.0.status();
                        match e.0.localize(fluent_locale.as_ref()) {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::error!("{e}");
                                exception_status = 500;
                                fallback_response(&fluent_locale)
                            }
                        }
                    }
                    None => {
                        exception_status = 500;
                        fallback_response(&fluent_locale)
                    }
                })
                .unwrap(),
            );
            ServiceResponse::new(
                new_req,
                HttpResponse::Ok().content_type(ContentType::json()).body(body),
            )
            .map_into_right_body()
        } else {
            ServiceResponse::new(new_req, res).map_into_left_body()
        };
        tracing::error!("Exception response: {exception_status} for {req_path}.");
        Ok(new_res)
    });

    Ok(ErrorHandlerResponse::Future(x))
}

fn fallback_response(locale: &AliceFluentLocale) -> LocalizedMsg {
    LocalizedMsg {
        status: 500,
        message: match locale.text("internal-error") {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("No any internal-error locale - {e}");
                "Internal error".to_string()
            }
        },
        content: None,
    }
}

fn localized_msg_to_service_response<B>(
    req: HttpRequest,
    localized_msg: LocalizedMsg,
) -> ServiceResponse<EitherBody<B>> {
    let json = serde_json::to_string(&localized_msg).unwrap();
    let response = HttpResponse::Ok().content_type(ContentType::json()).body(json);
    ServiceResponse::new(req, response).map_into_right_body()
}
