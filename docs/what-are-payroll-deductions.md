# What are payroll deductions?

**Who this is for:** Anyone who receives a paycheque and wants to know where the money went, without reading the CRA formula guide.

**When you finish:** You can name the main withholdings on a typical Canadian pay stub and try a live Ontario example.

Your employer pays you **gross** pay: the salary or wages for the period before withholdings.

Payroll deductions are amounts your employer withholds and remits on your behalf. On most Canadian cheques outside Quebec, the big ones are:

1. **Income tax** (federal and provincial or territorial)
2. **CPP** (Canada Pension Plan employee contribution)
3. **EI** (Employment Insurance premium)

**Net pay** is what lands in your account.

```text
gross pay − income tax − CPP − EI = net pay
```

(Your stub may show other lines, such as benefits or union dues. Those are separate from the CRA formula stack.)

The Canada Revenue Agency publishes the math in T4127 (Payroll Deductions Formulas). Takehome implements that guide for integrators. You do not need the guide to understand the headline equation above.

## Try it

Open the [Ontario calculator](https://takehome.gautamkhosla.com/calculators/on/). Enter a gross amount and see federal tax, provincial tax, CPP, EI, and net update in the browser. The page runs the same engine as the API and does not send your salary to our servers after the first load.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
