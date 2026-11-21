//! The same-build gate of the utility requests
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` AC-11, D-2026-11-20-4).
//!
//! A transfer session compares the two ends' contract versions at HELLO.
//! The utility requests are not sessions, so each carries the asker's
//! contract version and the daemon compares it before it reads or changes
//! anything ([`check`]). A daemon of an earlier contract does not serve
//! the request name at all: it answers "unimplemented", which the asker
//! reports as the same refusal ([`refusal`]).

use crate::transfer_session::CONTRACT_VERSION;

/// The daemon's half: refuse an asker of another contract, in the words a
/// transfer's refusal uses. `Ok` when the two speak the same one.
pub fn check(asker: u32) -> Result<(), tonic::Status> {
    if asker == CONTRACT_VERSION {
        return Ok(());
    }
    Err(tonic::Status::failed_precondition(mismatch(
        CONTRACT_VERSION,
        asker,
    )))
}

/// The refusal's words: this end's contract and the other's.
fn mismatch(daemon: u32, asker: u32) -> String {
    format!(
        "contract version mismatch (D-2026-08-18-2): daemon v{daemon} vs client v{asker}; \
         both must be the same blit release"
    )
}

/// The asker's half: what `status` from a utility request `verb` means.
/// A daemon that does not know the request at all is a daemon of another
/// contract; every other status is the daemon's own message.
pub fn refusal(verb: &str, status: tonic::Status) -> eyre::Report {
    if status.code() == tonic::Code::Unimplemented {
        return eyre::eyre!(
            "the daemon does not serve `{verb}` as this blit asks for it: it speaks another \
             contract version than this one (v{CONTRACT_VERSION}); both must be the same blit \
             release"
        );
    }
    eyre::eyre!(status.message().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC-11: the daemon answers an asker of its own contract and refuses
    /// one of any other, older or newer, naming both versions; and an
    /// asker told "unimplemented" — a daemon too old to know the request —
    /// reports the same thing, naming the verb.
    #[test]
    fn a_utility_request_is_answered_only_between_the_same_contract() {
        check(CONTRACT_VERSION).expect("the same contract");
        for other in [CONTRACT_VERSION - 1, CONTRACT_VERSION + 1, 0] {
            let status = check(other).expect_err("another contract");
            assert_eq!(status.code(), tonic::Code::FailedPrecondition);
            assert!(
                status
                    .message()
                    .contains(&format!("daemon v{CONTRACT_VERSION}"))
                    && status.message().contains(&format!("client v{other}")),
                "{}",
                status.message()
            );
            // The asker shows the daemon's own words.
            assert_eq!(refusal("du", status.clone()).to_string(), status.message());
        }
        let old_daemon = refusal("du", tonic::Status::unimplemented("unknown method Du"));
        let said = old_daemon.to_string();
        assert!(
            said.contains("`du`") && said.contains("another contract version"),
            "{said}"
        );
        assert!(said.contains(&format!("v{CONTRACT_VERSION}")), "{said}");
        // Any other status is the daemon's message, unchanged.
        assert_eq!(
            refusal("df", tonic::Status::not_found("no such module")).to_string(),
            "no such module"
        );
    }
}
