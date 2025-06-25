// Debug script to test different message formats

function testMessageFormats() {
  const payAddress = "CaskZcEG28E9F1qgEsw4bgpWExW7VFWx88Dy1EBF4FK4";
  const icPrincipal = "42s3u-zifnr-er5mr-5o4di-udavb-jhxk7-g6s3t-le2ft-hqtfc-b6oqe-xqe";
  const timestamp = Date.now();

  const messages = [
    {
      name: "Original (what frontend sends)",
      message: {
        pay_token: "SOL",
        pay_amount: "1000000",
        pay_address: payAddress,
        receive_token: "ckUSDT",
        receive_amount: "131188",
        receive_address: icPrincipal,
        max_slippage: 81.4,
        timestamp: timestamp,
        referred_by: null
      }
    },
    {
      name: "With ksUSDT token",
      message: {
        pay_token: "SOL",
        pay_amount: "1000000",
        pay_address: payAddress,
        receive_token: "ksUSDT",
        receive_amount: "131188",
        receive_address: icPrincipal,
        max_slippage: 81.4,
        timestamp: timestamp,
        referred_by: null
      }
    },
    {
      name: "Empty pay_address (backend extracts)",
      message: {
        pay_token: "SOL",
        pay_amount: "1000000",
        pay_address: "",
        receive_token: "ksUSDT",
        receive_amount: "131188",
        receive_address: icPrincipal,
        max_slippage: 81.4,
        timestamp: timestamp,
        referred_by: null
      }
    },
    {
      name: "Zero receive_amount",
      message: {
        pay_token: "SOL",
        pay_amount: "1000000",
        pay_address: payAddress,
        receive_token: "ksUSDT",
        receive_amount: "0",
        receive_address: icPrincipal,
        max_slippage: 81.4,
        timestamp: timestamp,
        referred_by: null
      }
    },
    {
      name: "Default slippage 1.0",
      message: {
        pay_token: "SOL",
        pay_amount: "1000000",
        pay_address: payAddress,
        receive_token: "ksUSDT",
        receive_amount: "131188",
        receive_address: icPrincipal,
        max_slippage: 1.0,
        timestamp: timestamp,
        referred_by: null
      }
    }
  ];

  console.log("Testing different message formats:\n");
  messages.forEach((test, index) => {
    const jsonMessage = JSON.stringify(test.message);
    console.log(`${index + 1}. ${test.name}:`);
    console.log(`   Message: ${jsonMessage}`);
    console.log(`   Length: ${jsonMessage.length} bytes`);
    console.log(`   First 50 bytes: [${new TextEncoder().encode(jsonMessage).slice(0, 50).join(', ')}]`);
    console.log();
  });
}

testMessageFormats();