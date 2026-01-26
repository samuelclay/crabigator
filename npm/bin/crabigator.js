#!/usr/bin/env node

/**
 * Crabigator NPM wrapper
 *
 * Detects the current platform and architecture, then spawns the appropriate
 * pre-built binary from the vendor directory.
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');

// Map Node.js platform/arch to our binary naming convention
const platformMap = {
  'darwin': 'darwin',
  'linux': 'linux',
  'win32': 'win32'
};

const archMap = {
  'arm64': 'arm64',
  'x64': 'x64'
};

function getBinaryPath() {
  const platform = platformMap[process.platform];
  const arch = archMap[process.arch];

  if (!platform || !arch) {
    console.error(`Unsupported platform: ${process.platform}-${process.arch}`);
    console.error('Crabigator supports: darwin-arm64, darwin-x64, linux-arm64, linux-x64, win32-arm64, win32-x64');
    process.exit(1);
  }

  const binaryName = process.platform === 'win32' ? 'crabigator.exe' : 'crabigator';
  const binaryPath = path.join(__dirname, '..', 'vendor', `${platform}-${arch}`, binaryName);

  if (!fs.existsSync(binaryPath)) {
    console.error(`Binary not found: ${binaryPath}`);
    console.error('This may indicate an incomplete installation. Try reinstalling:');
    console.error('  npm install -g crabigator@latest');
    process.exit(1);
  }

  return binaryPath;
}

function main() {
  const binaryPath = getBinaryPath();

  // Set environment variable to indicate installation method
  const env = { ...process.env, CRABIGATOR_INSTALLED_VIA: 'npm' };

  // Spawn the binary with all arguments passed through
  const child = spawn(binaryPath, process.argv.slice(2), {
    stdio: 'inherit',
    env
  });

  // Forward signals to child process
  const signals = ['SIGINT', 'SIGTERM', 'SIGHUP'];
  signals.forEach(signal => {
    process.on(signal, () => {
      if (!child.pid) return;
      try {
        process.kill(child.pid, signal);
      } catch (_) {
        // Ignore if the child already exited.
      }
    });
  });

  // Exit with child's exit code
  child.on('exit', (code, signal) => {
    if (signal) {
      // Child was killed by signal, exit with 128 + signal number
      const signalNumber = os.constants.signals[signal];
      if (typeof signalNumber === 'number') {
        process.exit(128 + signalNumber);
      }
      process.exit(128);
    }
    process.exit(code || 0);
  });

  child.on('error', (err) => {
    console.error(`Failed to start crabigator: ${err.message}`);
    process.exit(1);
  });
}

main();
