# Five-minute test

A developer who has never seen this repo lands on the site, finds the docs,
gets a key, and gets a correct authenticated answer. Stopwatch on. No
coaching. Pass is **5:00 or under** and `employee.net_pay` is `800.79`.

This file is the script, not a description of one.

## Script

Use a clean browser profile (no Takehome tabs, no saved passwords for this
site). A terminal is allowed — this is an API.

1. Start the stopwatch.
2. Open https://takehome.gautamkhosla.com/
3. Find the API docs. They are labelled **Docs** in the header, next to the
   Takehome name. Do not hunt the footer.
4. Open Docs. Read until you know how to get a key and what the first call is.
   The first call is Ontario weekly `$1000.00` on `2026-01-15`, claim code 1.
   The correct `employee.net_pay` is `800.79`.
5. Follow **Get a key**. Enter any email you control. Submit.
6. Verify:
   - If the page shows a verification link, open it.
   - Otherwise open the email from Takehome and open the link.
7. Copy the `np_test_` key. Leave the live key alone for this test.
8. In the terminal, run the first-call command from the docs page, with your
   test key in the `Authorization` header. On the verify page the same command
   is filled in with the key.
9. Confirm the JSON has `"net_pay": "800.79"`.
10. Stop the stopwatch.

### First call (the answer you must see)

```bash
curl -sS -X POST https://takehome.gautamkhosla.com/v1/deductions \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer np_test_YOUR_KEY' \
  -d '{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}'
```

`employee.net_pay` must be `800.79`. Any other figure is a fail even if the
clock is under five minutes.

## Local stand-in (when takehome.gautamkhosla.com is not the server under test)

Production URLs above are the product. Until they are deployed, run the same
script against a local stack that serves the same pages and the same Worker
handler:

```bash
# terminal 1 — default port 8787; set PORT if that bind is taken
cd services/api && npm run local

# terminal 2 — if the API is not on 8787, rebuild so the site targets it
cd web/site && PUBLIC_API_ORIGIN=http://127.0.0.1:8787 npm run build
npx astro preview --host 127.0.0.1 --port 4321
```

Then the script is identical with these substitutions:

- Site: http://127.0.0.1:4321/
- API: http://127.0.0.1:8787 (or `$PORT`)
- The local API echoes a verification link in the signup response so the test
  does not depend on inbound email.

Record that the run was local in the table.

## Observer notes (the second run)

Someone who writes code and has never seen this project runs the script.
The author watches and does not speak. Every pause goes in the table: the
step number, what they were looking for, how long they looked.

A classmate on a clean laptop is the real observer. A second operator who
did not implement the pages is a stand-in, not a substitute.

## Runs

| When | Who | Stack | Time | `net_pay` | Pass? | Pauses |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-09-16 | Author, first live walk | local site :4321 + API :8788 | 1:06 | 800.79 | yes | Step 6: signup status said “Check your email” while the verify link was already on the page. Looked for an inbox that does not exist. Docs itself was in the header on the first screen. |
| 2026-09-16 | Author, re-run after copy fix | same local stack | under 5:00 (path still 800.79; signup now says “Open the verification link. Keys are on the next page.”) | 800.79 | yes | Header Docs is visible next to the Netpay name. The province code row is still the noisiest thing on the landing page, but it is not on the way to Docs. |
| 2026-09-16 | Stand-in operator who did not implement the pages | local site :4321 + API :8788 | 0:58 | 800.79 | yes | Step 2: ~5s blank first load. Step 3: ~11s from start scanning the header; Docs was there, next to API keys and Pricing. Step 4: ~5s for `/docs/` to settle. Step 5: ~5s for the signup form; submit was immediate. Step 6: ~2s on “Sending…” then “Open the verification link” (not a blocker). Step 7: none; `np_test_` key on the verify page. Step 8: ~10s reading the truncated first-call curl, then ran it against :8788. |
| _date_ | Classmate on a clean laptop (still required) | https://takehome.gautamkhosla.com | | | | Production is live. A stand-in and an automated smoke are not a substitute. Watch without helping. Fill this row. |
