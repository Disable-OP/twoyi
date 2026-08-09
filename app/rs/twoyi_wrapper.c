/*
 * Copyright Disclaimer: AI-Generated Content
 * This file was created by GitHub Copilot, an AI coding assistant.
 * AI-generated content is not subject to copyright protection and is provided
 * without any warranty, express or implied, including warranties of merchantability,
 * fitness for a particular purpose, or non-infringement.
 * Use at your own risk.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/* 
 * Wrapper executable for libtwoyi.so
 * This allows the library to be executed from command line
 * Compile with: aarch64-linux-android-clang twoyi_wrapper.c -o twoyi -pie -fPIE
 */

int main(int argc, const char **argv);

/* The actual entry point - calls the main function from libtwoyi.so */
int _start() {
    /* Get argc and argv from the stack (ARM64 calling convention) */
    int argc;
    const char **argv;
    
    /* On ARM64, argc is at sp, argv is at sp+8 */
    __asm__ volatile(
        "ldr %w0, [sp]\n"      /* Load argc from stack */
        "add %1, sp, #8\n"     /* argv is 8 bytes after sp */
        : "=r"(argc), "=r"(argv)
    );
    
    /* Call the main function */
    return main(argc, argv);
}
