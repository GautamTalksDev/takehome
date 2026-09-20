# Glossary of factor symbols

**Who this is for:** Developers reading API `breakdown` fields or CONFORMANCE vectors who need plain English for each symbol.

**When you finish:** You can map any factor symbol in a response to its T4127 (Payroll Deductions Formulas) step and open the CRA payroll formulas hub.

Amounts in API responses are decimal strings in cents; this page defines names only.

## A

Annual taxable income.

**Source:** T4127 Chapter 4, Step 1

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## R

Federal tax rate that applies to the annual taxable income A.

**Source:** T4127 Chapter 8, Table 8.1

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## K

Federal constant; the tax overcharged when applying the higher federal rates to A.

**Source:** T4127 Chapter 8, Table 8.1

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## K1

Federal non-refundable personal tax credit (lowest federal tax rate × TC).

**Source:** T4127 Chapter 4, Step 2

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## K2

Federal tax credit for base CPP contributions and EI premiums for the year.

**Source:** T4127 Chapter 4, Step 2

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## K4

Federal non-refundable tax credit calculated using the Canada employment amount.

**Source:** T4127 Chapter 4, Step 2

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## T3

Annual basic federal tax.

**Source:** T4127 Chapter 4, Step 2

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## T1

Annual federal tax deduction.

**Source:** T4127 Chapter 4, Step 3

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## V

Provincial or territorial tax rate for the year.

**Source:** T4127 Chapter 8, Table 8.1

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## KP

Provincial or territorial constant.

**Source:** T4127 Chapter 8, Table 8.1

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## K1P

Provincial or territorial non-refundable personal tax credit (lowest provincial rate × TCP).

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## K2P

Provincial or territorial tax credit for base CPP contributions and EI premiums for the year.

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## T4

Annual basic provincial or territorial tax.

**Source:** T4127 Chapter 4, Step 4

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## V1

Provincial surtax calculated on the basic provincial tax (Ontario only).

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## V2

Ontario Health Premium calculated on taxable income.

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## S

Provincial tax reduction (Ontario and British Columbia).

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## T2

Annual provincial or territorial tax deduction (except Quebec).

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## T

Estimated federal and provincial or territorial tax deductions for the pay period: round((T1+T2)/P)+L. This is the combined Step 6 quotient. It may differ by one cent from employee.total_tax, which is the sum of the separately rounded federal and provincial lines PDOC displays (round(T1/P)+L and round(T2/P)). Compare screen totals to employee.total_tax; keep breakdown.T as the T4127 figure.

**Source:** T4127 Chapter 4, Step 6

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## TB

Tax on the current bonus or retroactive pay: the difference between annual tax (T1+T2) with the non-periodic payment and without it, floored at zero. If A with the bonus is at or below $5,000, a flat 15% (10% Quebec) of the bonus is used instead.

**Source:** T4127 Chapter 4, bonuses and retroactive pay

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## M

Year-to-date federal and provincial tax already deducted on periodic pay (Option 2). Does not include extra tax requested (L) or tax on bonuses (M1).

**Source:** T4127 Chapter 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## M1

Accumulated federal and provincial tax deductions on non-periodic payments such as bonuses, to the last pay period (Option 2).

**Source:** T4127 Chapter 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## C

Canada (or Quebec) Pension Plan contributions for the pay period.

**Source:** T4127 Chapter 6

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## C2

Second additional Canada (or Quebec) Pension Plan contributions for the pay period.

**Source:** T4127 Chapter 6

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## EI

Employment insurance premiums for the pay period.

**Source:** T4127 Chapter 7

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## F5

Deductions for Canada Pension Plan additional contributions for the pay period.

**Source:** T4127 Chapter 4, Step 1

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## BPAF

Federal Basic Personal Amount.

**Source:** T4127 Chapter 2; Table 8.2

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## K3

Other federal non-refundable tax credits authorized by a tax services office or tax centre.

**Source:** T4127 Chapter 4, Step 2

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## K3P

Other provincial or territorial non-refundable tax credits authorized by a tax services office or tax centre.

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## K4P

Territorial non-refundable tax credit calculated using the provincial or territorial Canada employment amount.

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## K5P

Provincial supplemental non-refundable tax credit (lowest provincial tax rate).

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## LCF

Federal labour-sponsored funds tax credit.

**Source:** T4127 Chapter 4, Step 3

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## LCP

Provincial or territorial labour-sponsored funds tax credit.

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## Y

Additional Ontario tax-reduction amount based on the number of eligible dependants, used in Factor S.

**Source:** T4127 Chapter 4, Step 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## F5A

CPP additional-contribution deduction for the pay period taken from periodic income.

**Source:** T4127 Chapter 4, Step 1

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## F5B

CPP additional-contribution deduction for the pay period taken from the non-periodic payment.

**Source:** T4127 Chapter 4, Step 1

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## D

Employee’s year-to-date (before the pay period) Canada Pension Plan contribution with the employer.

**Source:** T4127 Chapter 6

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## D1

Employee’s year-to-date (before the pay period) employment insurance premium with the employer.

**Source:** T4127 Chapter 7

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## D2

Employee’s year-to-date (before the pay period) second additional CPP contribution with the employer.

**Source:** T4127 Chapter 6

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## P

The number of pay periods in the year.

**Source:** T4127 Chapter 4, Step 1

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## PR

The number of pay periods left in the year (including the current pay period).

**Source:** T4127 Chapter 4, Step 1

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## PM

Total months during which CPP and/or QPP contributions are required (used to prorate the maximum contribution).

**Source:** T4127 Chapter 6

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## S1

Option 2 cumulative-averaging pair: total pay periods / current pay-period number (for example 52/1, 52/2). Never a binary float. Option 1 serializes as P/1.

**Source:** T4127 Chapter 5

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## CEA

Canada Employment Amount, a non-refundable tax credit used in the calculation for K4 and K4P.

**Source:** T4127 Chapter 8, Table 8.2

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## TC

Total claim amount reported on federal Form TD1.

**Source:** T4127 Chapter 2

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## TCP

Total claim amount reported on the provincial or territorial Form TD1.

**Source:** T4127 Chapter 2

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## IE

Insurable earnings for the pay period, including insurable taxable benefits, bonuses, and retroactive pay increases.

**Source:** T4127 Chapter 7

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

## QPIP

Quebec Parental Insurance Plan premium for the pay period.

**Source:** T4127 Chapter 8, Table 8.29

**CRA:** https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html

**Last reviewed:** 2026-09-20
**Engine:** 0.1.0
