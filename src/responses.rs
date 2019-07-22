#[allow(non_camel_case_types)]
pub enum Response<'a> {
    _211_SystemStatus,
    _214_Help,
    _220_ServiceReady(&'a str),
    _221_ServiceClosing,
    _235_AuthenticationSuccessful,
    _250_Completed(&'a str),
    _251_UserNotLocal,
    _252_CannotVRFYuser, // but will accept message and attempt delivery
    _334_Authenticate,
    _354_StartMailInput, // end with <CRLF>.<CRLF>
    _421_ServiceNotAvailable(&'a str),
    _450_MailboxUnavailable,
    _451_ErrorInProcessing,
    _452_InsufficientStorage,
    _455_ServerUnableToAccommodate,
    _500_SyntaxError, // command unrecognized
    _501_SyntaxErrorInParameters,
    _502_CommandNotImplemented,
    _503_BadSequence,
    _504_ParameterNotImplemented,
    _550_MailboxUnavailable,
    _551_UserNotLocal, // please try <forward-path> (See Section 3.4)
    _552_ExceededStorageAllocation,
    _553_MailboxNameNotAllowed,
    _554_TransactionFailed,
    _555_ParametersNotRecognized, // MAIL FROM/RCPT TO
}

impl<'a> Response<'a> {
    pub fn as_string(&self) -> String {
        match self {
            Response::_211_SystemStatus => "211".to_string(),
            Response::_214_Help => "214".to_string(),
            Response::_220_ServiceReady(domain) => {
                format!("220 local ESMTP {} Service Ready", domain)
            }
            Response::_221_ServiceClosing => "221 Bye".to_string(),
            Response::_235_AuthenticationSuccessful => "235 Authentication successful".to_string(),
            Response::_250_Completed(greeting) => format!("250 {}", greeting),
            Response::_251_UserNotLocal => "251".to_string(),
            Response::_252_CannotVRFYuser => "252".to_string(),
            Response::_334_Authenticate => "334 ".to_string(),
            Response::_354_StartMailInput => "354 End data with <CR><LF>.<CR><LF>".to_string(),
            Response::_421_ServiceNotAvailable(_domain) => "421".to_string(),
            Response::_450_MailboxUnavailable => "450".to_string(),
            Response::_451_ErrorInProcessing => "451".to_string(),
            Response::_452_InsufficientStorage => "452".to_string(),
            Response::_455_ServerUnableToAccommodate => "455".to_string(),
            Response::_500_SyntaxError => "500".to_string(),
            Response::_501_SyntaxErrorInParameters => "501".to_string(),
            Response::_502_CommandNotImplemented => "502".to_string(),
            Response::_503_BadSequence => "503".to_string(),
            Response::_504_ParameterNotImplemented => "504".to_string(),
            Response::_550_MailboxUnavailable => "550".to_string(),
            Response::_551_UserNotLocal => "551".to_string(),
            Response::_552_ExceededStorageAllocation => "552".to_string(),
            Response::_553_MailboxNameNotAllowed => "553".to_string(),
            Response::_554_TransactionFailed => "554".to_string(),
            Response::_555_ParametersNotRecognized => "555".to_string(),
        }
    }
}
