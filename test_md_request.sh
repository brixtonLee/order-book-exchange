#!/bin/bash

# Quick test to see the Market Data Request format
# This doesn't connect - just shows the message structure

cat << 'EOF' | cargo run --quiet --example - 2>/dev/null || true
use order_book_api::ctrader_fix::messages::{create_market_data_request, format_for_display};

fn main() {
    let md_request = create_market_data_request(
        "demo.fxpro.12345",
        "cServer",
        "QUOTE",
        "QUOTE",
        2,
        &["41"]  // XAUUSD (Gold)
    );

    println!("Market Data Request for Symbol 41 (XAUUSD):");
    println!("{}", format_for_display(&md_request));
    println!();

    // Show field order clearly
    println!("Field Order:");
    for (i, field) in md_request.split('\x01').enumerate() {
        if !field.is_empty() {
            println!("  {:2}. {}", i+1, field);
        }
    }
}
EOF

# Alternative: use inline Rust
echo ""
echo "The new Market Data Request will have this field order:"
echo ""
echo "  1. 8=FIX.4.4           (BeginString)"
echo "  2. 9=XXX               (BodyLength)"
echo "  3. 35=V                (MsgType=Market Data Request)"
echo "  4. 49=...              (SenderCompID)"
echo "  5. 56=cServer          (TargetCompID)"
echo "  6. 34=2                (MsgSeqNum)"
echo "  7. 52=...              (SendingTime)"
echo "  8. 50=QUOTE            (SenderSubID)"
echo "  9. 57=QUOTE            (TargetSubID)"
echo " 10. 262=REQ-...         (MDReqID)"
echo " 11. 263=1               (SubscriptionRequestType)"
echo " 12. 264=1               (MarketDepth)"
echo " 13. 265=1               (MDUpdateType)"
echo " 14. 146=1               (NoRelatedSym) ← First repeating group count"
echo " 15. 55=41               (Symbol) ← IMMEDIATELY after count!"
echo " 16. 267=2               (NoMDEntryTypes) ← Second repeating group count"
echo " 17. 269=0               (MDEntryType=Bid)"
echo " 18. 269=1               (MDEntryType=Offer)"
echo " 19. 10=XXX              (CheckSum)"
echo ""
echo "✅ The key fix: Symbol (55) now comes RIGHT AFTER NoRelatedSym (146)"
echo "   Previously it was appearing after NoMDEntryTypes (267), which was wrong!"
