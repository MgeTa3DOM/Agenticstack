const http = require('http');

const options = {
  hostname: 'localhost',
  port: 3000,
  path: '/',
  method: 'GET'
};

const req = http.request(options, (res) => {
  if (res.statusCode !== 200) {
    console.error(`Test failed: Expected status code 200, got ${res.statusCode}`);
    process.exit(1);
  }

  let data = '';
  res.on('data', (chunk) => {
    data += chunk;
  });

  res.on('end', () => {
    if (data.includes('Agentic Co-Work Engineering Dashboard')) {
      console.log('Test passed: Server is running and returning the correct HTML content.');
      process.exit(0);
    } else {
      console.error('Test failed: Expected HTML content not found in the response.');
      process.exit(1);
    }
  });
});

req.on('error', (error) => {
  console.error(`Test failed: Could not connect to the server. Error: ${error.message}`);
  process.exit(1);
});

req.end();
