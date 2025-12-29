# ⚠️ IMPORTANT: cTrader FIX API Password Setup

## 🔴 Error You're Seeing

```
║ [ 58] Unknown              = RET_INVALID_DATA               ║
```

**This means: WRONG PASSWORD!**

## ❌ Common Mistake

You're using your **account password** (`lkw06182000`), but cTrader FIX API requires a **separate FIX API password**.

**The FIX API password is NOT the same as your cTrader account password!**

---

## ✅ How to Set Your FIX API Password

### Method 1: cTrader Desktop App (Mac)

1. **Open cTrader Desktop** on your Mac
2. Click **Settings** (⚙️ icon in bottom-left corner)
3. Select **FIX API** from the menu
4. Click **"Change password"**
5. **Set a new password** for FIX API (can be different from your account password)
6. **Copy** or **write down** this password

### Method 2: cTrader Web

1. Go to https://ct.ctrader.com/ and login
2. Click **Settings** → **Advanced** → **FIX API**
3. Click **"Change password"**
4. Set your FIX API password

---

## 📋 Your FIX API Credentials

After setting the password, your credentials should be:

```
Price Connection (Market Data):
├─ Host: live-uk-eqx-01.p.c-trader.com
├─ Port: 5201 (Plain) or 5211 (SSL)
├─ SenderCompID: live.fxpro.8244184
├─ TargetCompID: cServer
├─ SenderSubID: QUOTE
├─ TargetSubID: QUOTE
├─ Username: 8244184
└─ Password: [YOUR_FIX_API_PASSWORD] ← NOT your account password!
```

---

## 🚀 Test Again

Once you've set the FIX API password:

```bash
cargo run --bin ctrader_fix_test
```

**When prompted, enter your FIX API password** (not your account password `lkw06182000`).

---

## 🔍 Verify Password is Set

In cTrader Desktop:
1. Go to **Settings** → **FIX API**
2. You should see:
   - Username: `8244184`
   - Password: `********` (hidden)
   - A "Change password" button

If the password field is empty or you've never set it, **you must set it first**.

---

## 💡 Pro Tip

The FIX API password can be the same as your account password if you want, but:
- It must be **explicitly set** in the FIX API settings
- Simply using your account password won't work unless it's been set as the FIX API password

---

## 📖 Official Documentation

From cTrader docs:
> "RET_INVALID_DATA indicates a wrong password. The FIX session password is NOT THE SAME as your general CTID password."

Source: https://help.ctrader.com/fix/getting-credentials/

---

**Next Steps:**
1. ✅ Open cTrader Desktop/Web
2. ✅ Set FIX API password in Settings → FIX API
3. ✅ Run the test again with the correct password
4. ✅ You should see market data! 🎉
