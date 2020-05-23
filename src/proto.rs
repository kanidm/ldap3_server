use lber::common::TagClass;
use lber::structure::{StructureTag, PL};
use lber::structures::ASNTag;
use lber::structures::{Enumerated, Integer, Null, OctetString, Sequence, Tag};
use lber::universal::Types;
use std::convert::{From, TryFrom};
use std::iter::once_with;

#[derive(Debug, Clone, PartialEq)]
pub struct LdapMsg {
    pub msgid: i32,
    pub op: LdapOp,
    pub ctrl: Vec<()>,
}

#[derive(Debug, Clone, PartialEq)]
#[repr(i64)]
pub enum LdapResultCode {
    Success = 0,
    OperationsError = 1,
    ProtocolError = 2,
    TimeLimitExceeded = 3,
    SizeLimitExceeded = 4,
    CompareFalse = 5,
    CompareTrue = 6,
    AuthMethodNotSupported = 7,
    StrongerAuthRequired = 8,
    // 9 reserved?
    Referral = 10,
    AdminLimitExceeded = 11,
    UnavailableCriticalExtension = 12,
    ConfidentialityRequired = 13,
    SaslBindInProgress = 14,
    // 15 ?
    NoSuchAttribute = 16,
    UndefinedAttributeType = 17,
    InappropriateMatching = 18,
    ConstraintViolation = 19,
    AttributeOrValueExists = 20,
    InvalidAttributeSyntax = 21,
    //22 31
    NoSuchObject = 32,
    AliasProblem = 33,
    InvalidDNSyntax = 34,
    // 35
    AliasDereferencingProblem = 35,
    // 36 - 47
    InappropriateAuthentication = 48,
    InvalidCredentials = 49,
    InsufficentAccessRights = 50,
    Busy = 51,
    Unavailable = 52,
    UnwillingToPerform = 53,
    LoopDetect = 54,
    // 55 - 63
    NamingViolation = 64,
    ObjectClassViolation = 65,
    NotAllowedOnNonLeaf = 66,
    NotALlowedOnRDN = 67,
    EntryAlreadyExists = 68,
    ObjectClassModsProhibited = 69,
    // 70
    AffectsMultipleDSAs = 71,
    // 72 - 79
    Other = 80,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LdapResult {
    pub code: LdapResultCode,
    pub matcheddn: String,
    pub message: String,
    pub referral: Vec<()>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LdapOp {
    SimpleBind(LdapSimpleBind),
    BindResponse(LdapBindResponse),
    UnbindRequest,
    // https://tools.ietf.org/html/rfc4511#section-4.5
    // 3 -> SearchRequest
    // 4 -> SearchResultEntry
    // 5 -> SearchResultDone

    // https://tools.ietf.org/html/rfc4511#section-4.12
    ExtendedRequest(LdapExtendedRequest),
    ExtendedResponse(LdapExtendedResponse),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LdapSimpleBind {
    pub dn: String,
    pub pw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LdapBindResponse {
    pub res: LdapResult,
    pub saslcreds: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LdapExtendedRequest {
    // 0
    pub name: String,
    // 1
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LdapExtendedResponse {
    pub res: LdapResult,
    // 10
    pub name: Option<String>,
    // 11
    pub value: Option<String>,
}

impl LdapOp {
    pub fn is_simplebind(&self) -> bool {
        match self {
            LdapOp::SimpleBind(_) => true,
            _ => false,
        }
    }
}

impl LdapSimpleBind {
    pub fn new_anonymous() -> Self {
        LdapSimpleBind {
            dn: "".to_string(),
            pw: "".to_string(),
        }
    }
}

impl From<LdapSimpleBind> for Tag {
    fn from(value: LdapSimpleBind) -> Tag {
        Tag::Sequence(Sequence {
            id: 0,
            class: TagClass::Application,
            inner: vec![
                Tag::Integer(Integer {
                    inner: 3,
                    ..Default::default()
                }),
                Tag::OctetString(OctetString {
                    inner: Vec::from(value.dn),
                    ..Default::default()
                }),
                Tag::OctetString(OctetString {
                    id: 0,
                    class: TagClass::Context,
                    inner: Vec::from(value.pw),
                }),
            ],
        })
    }
}

impl LdapMsg {
    pub fn new(msgid: i32, op: LdapOp) -> Self {
        LdapMsg {
            msgid,
            op,
            ctrl: Vec::new(),
        }
    }
}

impl TryFrom<StructureTag> for LdapMsg {
    type Error = ();

    /// https://tools.ietf.org/html/rfc4511#section-4.1.1
    fn try_from(value: StructureTag) -> Result<Self, Self::Error> {
        /*
         * LDAPMessage ::= SEQUENCE {
         *      messageID       MessageID,
         *      protocolOp      CHOICE {
         *           bindRequest           BindRequest,
         *           bindResponse          BindResponse,
         *           unbindRequest         UnbindRequest,
         *           searchRequest         SearchRequest,
         *           searchResEntry        SearchResultEntry,
         *           searchResDone         SearchResultDone,
         *           searchResRef          SearchResultReference,
         *           modifyRequest         ModifyRequest,
         *           modifyResponse        ModifyResponse,
         *           addRequest            AddRequest,
         *           addResponse           AddResponse,
         *           delRequest            DelRequest,
         *           delResponse           DelResponse,
         *           modDNRequest          ModifyDNRequest,
         *           modDNResponse         ModifyDNResponse,
         *           compareRequest        CompareRequest,
         *           compareResponse       CompareResponse,
         *           abandonRequest        AbandonRequest,
         *           extendedReq           ExtendedRequest,
         *           extendedResp          ExtendedResponse,
         *           ...,
         *           intermediateResponse  IntermediateResponse },
         *      controls       [0] Controls OPTIONAL }
         *
         * MessageID ::= INTEGER (0 ..  maxInt)
         *
         * maxInt INTEGER ::= 2147483647 -- (2^^31 - 1) --
         */
        let mut seq = value
            .match_id(Types::Sequence as u64)
            .and_then(|t| t.expect_constructed())
            .ok_or(())?;

        // seq is now a vec of the inner elements.
        let (msgid_tag, op_tag, ctrl_tag) = match seq.len() {
            2 => {
                // We destructure in reverse order due to how vec in rust
                // works.
                let c = None;
                let o = seq.pop();
                let m = seq.pop();
                (m, o, c)
            }
            3 => {
                let c = seq.pop();
                let o = seq.pop();
                let m = seq.pop();
                (m, o, c)
            }
            _ => return Err(()),
        };

        // The first item should be the messageId
        let msgid = msgid_tag
            .and_then(|t| t.match_class(TagClass::Universal))
            .and_then(|t| t.match_id(Types::Integer as u64))
            // Get the raw bytes
            .and_then(|t| t.expect_primitive())
            .and_then(ber_integer_to_i64)
            // Trunc to i32.
            .map(|i| i as i32)
            .ok_or(())?;

        let op = op_tag.ok_or(())?;
        let op = LdapOp::try_from(op)?;

        let ctrl = ctrl_tag
            .and_then(|t| t.match_class(TagClass::Context))
            .and_then(|t| t.match_id(0))
            // So it's probably controls, decode them?
            .map(|_t| Vec::new())
            .unwrap_or_else(|| Vec::new());

        Ok(LdapMsg { msgid, op, ctrl })
    }
}

impl From<LdapMsg> for StructureTag {
    fn from(value: LdapMsg) -> StructureTag {
        let LdapMsg { msgid, op, ctrl } = value;
        let seq: Vec<_> = once_with(|| {
            Some(Tag::Integer(Integer {
                inner: msgid as i64,
                ..Default::default()
            }))
        })
        .chain(once_with(|| Some(op.into())))
        .chain(once_with(|| {
            if ctrl.len() > 0 {
                unimplemented!();
            } else {
                None
            }
        }))
        .filter_map(|v| v)
        .collect();
        Tag::Sequence(Sequence {
            inner: seq,
            ..Default::default()
        })
        .into_structure()
    }
}

impl TryFrom<StructureTag> for LdapOp {
    type Error = ();

    fn try_from(value: StructureTag) -> Result<Self, Self::Error> {
        let StructureTag { class, id, payload } = value;
        if class != TagClass::Application {
            return Err(());
        }
        match (id, payload) {
            // https://tools.ietf.org/html/rfc4511#section-4.2
            // BindRequest
            (0, PL::C(inner)) => LdapSimpleBind::try_from(inner).map(|v| LdapOp::SimpleBind(v)),
            // BindResponse
            (1, PL::C(inner)) => LdapBindResponse::try_from(inner).map(|v| LdapOp::BindResponse(v)),
            // UnbindRequest
            (2, _) => Ok(LdapOp::UnbindRequest),
            (23, PL::C(inner)) => {
                LdapExtendedRequest::try_from(inner).map(|v| LdapOp::ExtendedRequest(v))
            }
            (24, PL::C(inner)) => {
                LdapExtendedResponse::try_from(inner).map(|v| LdapOp::ExtendedResponse(v))
            }
            (id, _) => {
                println!("unknown op -> {:?}", id);
                Err(())
            }
        }
    }
}

impl From<LdapOp> for Tag {
    fn from(value: LdapOp) -> Tag {
        match value {
            LdapOp::SimpleBind(lsb) => Tag::Sequence(Sequence {
                class: TagClass::Application,
                id: 0,
                inner: lsb.into(),
            }),
            LdapOp::BindResponse(lbr) => Tag::Sequence(Sequence {
                class: TagClass::Application,
                id: 1,
                inner: lbr.into(),
            }),
            LdapOp::UnbindRequest => Tag::Null(Null {
                class: TagClass::Application,
                id: 2,
                inner: (),
            }),
            LdapOp::ExtendedRequest(ler) => Tag::Sequence(Sequence {
                class: TagClass::Application,
                id: 23,
                inner: ler.into(),
            }),
            LdapOp::ExtendedResponse(ler) => Tag::Sequence(Sequence {
                class: TagClass::Application,
                id: 24,
                inner: ler.into(),
            }),
        }
    }
}

impl TryFrom<Vec<StructureTag>> for LdapSimpleBind {
    type Error = ();

    fn try_from(mut value: Vec<StructureTag>) -> Result<Self, Self::Error> {
        // https://tools.ietf.org/html/rfc4511#section-4.2
        // BindRequest

        // We need 3 elements, the version, the dn, and a choice of the
        // credential (we only support simple)
        let (v, dn, choice) = if value.len() == 3 {
            // Remember it's a vec, so we pop in reverse order.
            let choice = value.pop();
            let dn = value.pop();
            let v = value.pop();
            (v, dn, choice)
        } else {
            return Err(());
        };

        // Check the version is 3
        let v = v
            .and_then(|t| t.match_class(TagClass::Universal))
            .and_then(|t| t.match_id(Types::Integer as u64))
            .and_then(|t| t.expect_primitive())
            .and_then(ber_integer_to_i64)
            .ok_or(())?;
        if v != 3 {
            return Err(());
        };

        // Get the DN
        let dn = dn
            .and_then(|t| t.match_class(TagClass::Universal))
            .and_then(|t| t.match_id(Types::OctetString as u64))
            .and_then(|t| t.expect_primitive())
            .and_then(|bv| String::from_utf8(bv).ok())
            .ok_or(())?;

        // Andddd get the password.
        let pw = choice
            .and_then(|t| t.match_class(TagClass::Context))
            // Only match pw
            .and_then(|t| t.match_id(0))
            .and_then(|t| t.expect_primitive())
            .and_then(|bv| String::from_utf8(bv).ok())
            .ok_or(())?;

        Ok(LdapSimpleBind { dn, pw })
    }
}

impl From<LdapSimpleBind> for Vec<Tag> {
    fn from(value: LdapSimpleBind) -> Vec<Tag> {
        vec![
            Tag::Integer(Integer {
                inner: 3,
                ..Default::default()
            }),
            Tag::OctetString(OctetString {
                inner: Vec::from(value.dn),
                ..Default::default()
            }),
            Tag::OctetString(OctetString {
                id: 0,
                class: TagClass::Context,
                inner: Vec::from(value.pw),
            }),
        ]
    }
}

impl LdapResult {
    fn into_tag_iter(self) -> impl Iterator<Item = Option<Tag>> {
        let LdapResult {
            code,
            matcheddn,
            message,
            referral,
        } = self;

        once_with(|| {
            Some(Tag::Enumerated(Enumerated {
                inner: code as i64,
                ..Default::default()
            }))
        })
        .chain(once_with(|| {
            Some(Tag::OctetString(OctetString {
                inner: Vec::from(matcheddn),
                ..Default::default()
            }))
        }))
        .chain(once_with(|| {
            Some(Tag::OctetString(OctetString {
                inner: Vec::from(message),
                ..Default::default()
            }))
        }))
        .chain(once_with(move || {
            if referral.len() > 0 {
                // Remember to mark this as id 3, class::Context  (I think)
                unimplemented!();
            } else {
                None
            }
        }))
    }
}

impl LdapResult {
    fn try_from_tag(mut value: Vec<StructureTag>) -> Result<(Self, Vec<StructureTag>), ()> {
        // First, reverse all the elements so we are in the correct order.
        value.reverse();

        let code = value
            .pop()
            .and_then(|t| t.match_class(TagClass::Universal))
            .and_then(|t| t.match_id(Types::Enumerated as u64))
            .and_then(|t| t.expect_primitive())
            .and_then(ber_integer_to_i64)
            .ok_or(())
            .and_then(|i| LdapResultCode::try_from(i))?;

        let matcheddn = value
            .pop()
            .and_then(|t| t.match_class(TagClass::Universal))
            .and_then(|t| t.match_id(Types::OctetString as u64))
            .and_then(|t| t.expect_primitive())
            .and_then(|bv| String::from_utf8(bv).ok())
            .ok_or(())?;

        let message = value
            .pop()
            .and_then(|t| t.match_class(TagClass::Universal))
            .and_then(|t| t.match_id(Types::OctetString as u64))
            .and_then(|t| t.expect_primitive())
            .and_then(|bv| String::from_utf8(bv).ok())
            .ok_or(())?;

        let (_referrals, other): (Vec<_>, Vec<_>) = value.into_iter().partition(|v| v.id == 3);

        // assert referrals only is one
        let referral = Vec::new();

        Ok((
            LdapResult {
                code,
                matcheddn,
                message,
                referral,
            },
            other,
        ))
    }
}

impl LdapBindResponse {
    pub fn new_success(msg: &str) -> Self {
        LdapBindResponse {
            res: LdapResult {
                code: LdapResultCode::Success,
                matcheddn: "".to_string(),
                message: msg.to_string(),
                referral: Vec::new(),
            },
            saslcreds: None,
        }
    }

    pub fn new_invalidcredentials(dn: &str, msg: &str) -> Self {
        LdapBindResponse {
            res: LdapResult {
                code: LdapResultCode::InvalidCredentials,
                matcheddn: dn.to_string(),
                message: msg.to_string(),
                referral: Vec::new(),
            },
            saslcreds: None,
        }
    }
}

impl TryFrom<Vec<StructureTag>> for LdapBindResponse {
    type Error = ();

    fn try_from(value: Vec<StructureTag>) -> Result<Self, Self::Error> {
        // This MUST be the first thing we do!
        let (res, _remtag) = LdapResult::try_from_tag(value)?;

        // Now with the remaining tags, populate anything else we need
        Ok(LdapBindResponse {
            res,
            saslcreds: None,
        })
    }
}

impl From<LdapBindResponse> for Vec<Tag> {
    fn from(value: LdapBindResponse) -> Vec<Tag> {
        // get all the values from the LdapResult
        let LdapBindResponse { res, saslcreds } = value;
        res.into_tag_iter()
            .chain(once_with(|| {
                saslcreds.map(|sc| {
                    Tag::OctetString(OctetString {
                        inner: Vec::from(sc),
                        ..Default::default()
                    })
                })
            }))
            .filter_map(|s| s)
            .collect()
    }
}

impl TryFrom<Vec<StructureTag>> for LdapExtendedRequest {
    type Error = ();

    fn try_from(mut value: Vec<StructureTag>) -> Result<Self, Self::Error> {
        // Put the values in order.
        value.reverse();
        // Read the values in
        let name = value
            .pop()
            .and_then(|t| t.match_class(TagClass::Context))
            .and_then(|t| t.match_id(0))
            .and_then(|t| t.expect_primitive())
            .and_then(|bv| String::from_utf8(bv).ok())
            .ok_or(())?;

        let value = value
            .pop()
            .and_then(|t| t.match_class(TagClass::Context))
            .and_then(|t| t.match_id(1))
            .and_then(|t| t.expect_primitive())
            .and_then(|bv| String::from_utf8(bv).ok());

        Ok(LdapExtendedRequest { name, value })
    }
}

impl From<LdapExtendedRequest> for Vec<Tag> {
    fn from(value: LdapExtendedRequest) -> Vec<Tag> {
        let LdapExtendedRequest { name, value } = value;

        once_with(|| {
            Tag::OctetString(OctetString {
                id: 0,
                class: TagClass::Context,
                inner: Vec::from(name),
            })
        })
        .chain(
            once_with(|| {
                value.map(|v| {
                    Tag::OctetString(OctetString {
                        id: 1,
                        class: TagClass::Context,
                        inner: Vec::from(v),
                    })
                })
            })
            .filter_map(|s| s),
        )
        .collect()
    }
}

impl TryFrom<Vec<StructureTag>> for LdapExtendedResponse {
    type Error = ();

    fn try_from(value: Vec<StructureTag>) -> Result<Self, Self::Error> {
        // This MUST be the first thing we do!
        let (res, remtag) = LdapResult::try_from_tag(value)?;
        // Now from the remaining tags, get the items.
        let mut name = None;
        let mut value = None;
        remtag.into_iter().for_each(|v| {
            match (v.id, v.class) {
                (10, TagClass::Context) => {
                    name = v
                        .expect_primitive()
                        .and_then(|bv| String::from_utf8(bv).ok())
                }
                (11, TagClass::Context) => {
                    value = v
                        .expect_primitive()
                        .and_then(|bv| String::from_utf8(bv).ok())
                }
                _ => {
                    // Do nothing
                }
            }
        });

        Ok(LdapExtendedResponse { res, name, value })
    }
}

impl From<LdapExtendedResponse> for Vec<Tag> {
    fn from(value: LdapExtendedResponse) -> Vec<Tag> {
        let LdapExtendedResponse { res, name, value } = value;
        res.into_tag_iter()
            .chain(once_with(|| {
                name.map(|v| {
                    Tag::OctetString(OctetString {
                        id: 10,
                        class: TagClass::Context,
                        inner: Vec::from(v),
                    })
                })
            }))
            .chain(once_with(|| {
                value.map(|v| {
                    Tag::OctetString(OctetString {
                        id: 11,
                        class: TagClass::Context,
                        inner: Vec::from(v),
                    })
                })
            }))
            .filter_map(|s| s)
            .collect()
    }
}

impl LdapExtendedResponse {
    pub fn new_success(name: Option<&str>, value: Option<&str>) -> Self {
        LdapExtendedResponse {
            res: LdapResult {
                code: LdapResultCode::Success,
                matcheddn: "".to_string(),
                message: "".to_string(),
                referral: Vec::new(),
            },
            name: name.map(|v| v.to_string()),
            value: value.map(|v| v.to_string()),
        }
    }

    pub fn new_operationserror(msg: &str) -> Self {
        LdapExtendedResponse {
            res: LdapResult {
                code: LdapResultCode::OperationsError,
                matcheddn: "".to_string(),
                message: msg.to_string(),
                referral: Vec::new(),
            },
            name: None,
            value: None,
        }
    }
}

impl TryFrom<i64> for LdapResultCode {
    type Error = ();

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(LdapResultCode::Success),
            _ => Err(()),
        }
    }
}

fn ber_integer_to_i64(bv: Vec<u8>) -> Option<i64> {
    // ints in ber are be and may be truncated.
    let mut raw: [u8; 8] = [0; 8];
    // This is where we need to start inserting bytes.
    let base = if bv.len() > 8 {
        return None;
    } else {
        8 - bv.len()
    };
    for i in 0..bv.len() {
        raw[base + i] = bv[i];
    }
    Some(i64::from_be_bytes(raw))
}